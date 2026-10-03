//! Clipboard preview is local; only an explicitly confirmed editor snapshot is saved.
use super::recipe_import_mapping::map_import_candidate;
use crate::{api::recipe::create_recipe_draft, components::AuthRequired, Route};
use dioxus::prelude::*;
use serde_json::Value;

const BRIDGE: Asset = asset!("/assets/recipe-import-bridge.mjs");
const PARSER: Asset = asset!("/scripts/recipe-import-parser.mjs");
// Track the external parser as a Rust build input so incremental asset builds
// cannot keep an old fingerprint after a JavaScript-only adapter change.
const _: &[u8] = include_bytes!("../../scripts/recipe-import-parser.mjs");
const _: &[u8] = include_bytes!("../../assets/recipe-import-bridge.mjs");

/// Choices come from the same local catalogue used by the worker's parser.
#[derive(Clone, PartialEq, serde::Deserialize)]
struct ImportProvider {
    id: String,
    label: String,
}

#[component]
fn ProviderSelector(
    provider_choices: Vec<ImportProvider>,
    selected_provider: String,
    disabled: bool,
    on_change: Option<EventHandler<String>>,
) -> Element {
    rsx! {
        label { r#for: "recipe-import-provider", "Recipe provider" }
        select {
            id: "recipe-import-provider", class: "input", width: "100%",
            value: selected_provider, disabled,
            onchange: move |event| { if let Some(handler) = on_change { handler.call(event.value()); } },
            option { value: "", "Automatic (URL or generic parser)" }
            for provider in provider_choices {
                option { key: "{provider.id}", value: provider.id, "{provider.label}" }
            }
        }
    }
}

#[component]
pub fn RecipeImport() -> Element {
    rsx! { AuthRequired { ImportForm {} } }
}

/// Keep the bridge mounted only within the authenticated page lifecycle.
#[component]
fn ImportForm() -> Element {
    let mut candidate = use_signal(|| None::<Value>);
    let mut message = use_signal(|| "Loading clipboard capture…".to_string());
    let mut ready = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut source = use_signal(String::new);
    let mut provider = use_signal(String::new);
    let mut provider_choices = use_signal(Vec::<ImportProvider>::new);
    let mut parsing = use_signal(|| false);
    let mut saving = use_signal(|| false);
    let mut keep_context = use_signal(|| true);
    let navigator = use_navigator();
    use_drop(move || {
        let _ = document::eval("window.nomsImportCleanup?.(); window.nomsImportCleanup = null;");
    });
    let mapped = candidate
        .read()
        .as_ref()
        .map(|value| map_import_candidate(value, keep_context()));
    rsx! {
        main { class: "container", max_width: "850px", padding_bottom: "var(--space-xl)",
            h1 { "Import a recipe" }
            p { "Copy the whole recipe page, then paste below. Parsing happens in your browser. NOMS does not fetch the source page." }
            ProviderSelector {
                provider_choices: provider_choices(), selected_provider: provider(), disabled: saving() || !ready(),
                on_change: move |value: String| {
                    provider.set(value); candidate.set(None); error.set(None); parsing.set(false);
                    let detail = serde_json::json!({"sourceUrl":source(),"providerId":provider()});
                    let _ = document::eval(&format!("document.getElementById('recipe-import-paste')?.dispatchEvent(new CustomEvent('noms-source-change', {{detail:{detail}}}));"));
                }
            }
            p { "Choose the recipe site to use its parser without a URL. Automatic uses the optional URL, or generic parsing when it is blank." }
            label { r#for: "recipe-import-source", "Recipe source URL (optional)" }
            input { id: "recipe-import-source", r#type: "url", class: "input", width: "100%", value: "{source}", disabled: saving() || !ready(),
                placeholder: "https://…",
                oninput: move |event| {
                    source.set(event.value()); candidate.set(None); error.set(None); parsing.set(false);
                    let detail = serde_json::json!({"sourceUrl":source(),"providerId":provider()});
                    let _ = document::eval(&format!("document.getElementById('recipe-import-paste')?.dispatchEvent(new CustomEvent('noms-source-change', {{detail:{detail}}}));"));
                }
            }
            p { "Optional: keep the original recipe link for attribution and relative image references. It must match the selected provider." }
            label { r#for: "recipe-import-paste", "Full clipboard paste" }
            textarea { id: "recipe-import-paste", rows: "9", width: "100%", disabled: saving() || !ready(),
                placeholder: "On the recipe page, select all and copy. Paste here.",
                onmounted: move |_| {
                    spawn(async move {
                        let script = format!("try {{ const el = document.getElementById('recipe-import-paste'); const {{mountClipboardImport}} = await import({}); const parserUrl = {}; const {{RECIPE_IMPORT_PROVIDERS}} = await import(parserUrl); if (el?.isConnected) window.nomsImportCleanup = mountClipboardImport(el, parserUrl, value => dioxus.send(value.type === 'ready' ? {{...value,providers:RECIPE_IMPORT_PROVIDERS}} : value)); }} catch(e) {{ dioxus.send({{type:'error',message:String(e)}}); }}", serde_json::to_string(&BRIDGE.to_string()).unwrap(), serde_json::to_string(&PARSER.to_string()).unwrap());
                        let mut bridge = document::eval(&script);
                        while let Ok(event) = bridge.recv::<Value>().await {
                            match event["type"].as_str() {
                                Some("ready") => {
                                    match serde_json::from_value::<Vec<ImportProvider>>(event["providers"].clone()) {
                                        Ok(choices) if !choices.is_empty() => { provider_choices.set(choices); ready.set(true); message.set("Choose a provider and paste a full recipe page to begin.".into()); }
                                        _ => { error.set(Some("Unable to load recipe provider choices. Refresh and try again.".into())); }
                                    }
                                }
                                Some("captured") => { candidate.set(None); error.set(None); parsing.set(false); message.set(if event["html"].as_bool().unwrap_or(false) { "HTML and text captured. Ready to preview." } else { "Text captured. Ready to preview." }.into()); }
                                Some("parsing") => { parsing.set(true); error.set(None); candidate.set(None); message.set("Parsing recipe…".into()); }
                                Some("result") => { parsing.set(false); candidate.set(Some(event["candidate"].clone())); message.set("Preview ready. Check the extracted content before creating a draft.".into()); }
                                Some("error") => { parsing.set(false); candidate.set(None); error.set(Some(event["message"].as_str().unwrap_or("Unable to parse this paste.").into())); }
                                _ => {}
                            }
                        }
                    });
                }
            }
            p { role: "status", aria_live: "polite", "{message}" }
            button { class: "btn btn-secondary", disabled: parsing() || saving() || !ready(),
                onclick: move |_| {
                    let detail = serde_json::json!({"sourceUrl":source(),"providerId":provider()});
                    let _ = document::eval(&format!("document.getElementById('recipe-import-paste')?.dispatchEvent(new CustomEvent('noms-preview', {{detail:{detail}}}));"));
                }, "Preview recipe"
            }
            if let Some(problem) = error() { p { role: "alert", "{problem}" } }
            if let Some(value) = candidate() {
                section { margin_top: "var(--space-xl)",
                    h2 { "Recipe preview" }
                    h3 { {display_value(&value["fields"]["title"]["value"])} }
                    p { {display_value(&value["fields"]["description"]["value"])} }
                    h3 { "Ingredients" }
                    if let Some(sections) = value["fields"]["ingredientSections"].as_array() {
                        for section in sections {
                            if let Some(name) = section["name"].as_str() { h4 { "{name}" } }
                            if let Some(items) = section["items"].as_array() { ul {
                                for item in items { li { {format!("{} · {} · {}", display_value(&item["amount"]), display_value(&item["unit"]), display_value(&item["name"]))} } }
                            } }
                        }
                    }
                    h3 { "Instructions" }
                    if let Some(sections) = value["fields"]["instructionSections"].as_array() {
                        for section in sections {
                            if let Some(name) = section["name"].as_str() { h4 { "{name}" } }
                            if let Some(steps) = section["steps"].as_array() { {render_steps(steps)} }
                        }
                    }
                    for (key, label) in [("equipment", "Equipment"), ("notes", "Notes")] {
                        h3 { "{label}" }
                        if let Some(items) = value["fields"][key].as_array() {
                            if items.is_empty() { p { "None" } }
                            for item in items { p { {display_value(&item["value"])} } }
                        }
                    }
                    for (key, label) in [("warnings", "Parser warnings"), ("unclassifiedLines", "Unclassified content"), ("competingCandidates", "Other recipe candidates")] {
                        details { summary { "{label}" }
                            pre { white_space: "pre-wrap", overflow_wrap: "anywhere", "{serde_json::to_string_pretty(&value[key]).unwrap_or_default()}" }
                        }
                    }
                    label { display: "block", margin_top: "var(--space-lg)",
                        input { r#type: "checkbox", checked: keep_context(), disabled: saving(), onchange: move |event| keep_context.set(event.checked()) }
                        " Keep source, group labels, total time, yield, original ingredient text and notes in commentary"
                    }
                    ImportMetadataPreview { candidate: value.clone(), keep_context: keep_context() }
                    if let Some(Err(problem)) = mapped.as_ref() { p { role: "alert", "{problem}" } }
                    button { class: "btn btn-primary", margin_top: "var(--space-md)", disabled: saving() || !matches!(mapped, Some(Ok(_))),
                        onclick: move |_| {
                            if saving() { return; }
                            let Some(value) = candidate() else { return; };
                            let Ok(editor) = map_import_candidate(&value, keep_context()) else { return; };
                            saving.set(true); error.set(None);
                            spawn(async move {
                                match create_recipe_draft(editor).await {
                                    Ok(draft) => { navigator.push(Route::RecipeDraftEdit { id: draft.id.to_string() }); }
                                    Err(problem) => { error.set(Some(format!("Could not create draft: {problem}"))); saving.set(false); }
                                }
                            });
                        },
                        if saving() { "Creating draft…" } else { "Confirm and create private draft" }
                    }
                    p { "You can edit the draft before publishing. Nothing is saved until you confirm." }
                }
            }
        }
    }
}

/// Show the exact mapped values and commentary that confirmation will save.
#[component]
fn ImportMetadataPreview(candidate: Value, keep_context: bool) -> Element {
    let mapped = map_import_candidate(&candidate, keep_context).ok();
    let number = |value: Option<i32>| {
        value
            .map(|value| value.to_string())
            .unwrap_or_else(|| "Missing".into())
    };
    rsx! {
        section {
            h3 { "Details to save" }
            p { "Prep time (minutes): " {number(mapped.as_ref().and_then(|editor| editor.prep_time_minutes))} }
            p { "Cook time (minutes): " {number(mapped.as_ref().and_then(|editor| editor.cook_time_minutes))} }
            p { "Servings: " {number(mapped.as_ref().and_then(|editor| editor.servings))} }
            for (key, label) in [("provider", "Provider"), ("suppliedUrl", "Source"), ("author", "Author"), ("attribution", "Attribution")] {
                p { "{label}: " {display_value(&candidate["source"][key])} }
            }
            h4 { "Original time and yield text" }
            for (key, label) in [("prepTime", "Prep time"), ("cookTime", "Cook time"), ("totalTime", "Total time"), ("yield", "Yield")] {
                p { "{label}: " {display_value(&candidate["fields"][key]["raw"])} }
            }
            h4 { "Commentary" }
            p { white_space: "pre-wrap", overflow_wrap: "anywhere", {mapped.as_ref().and_then(|editor| editor.commentary.as_deref()).unwrap_or("None")} }
            h4 { "Image references" }
            p { "Images are not loaded or imported in this version." }
            if let Some(images) = candidate["fields"]["images"].as_array() {
                if images.is_empty() { p { "None" } }
                for image in images {
                    p { overflow_wrap: "anywhere", {image["url"].as_str().or_else(|| image["sourceUrl"].as_str()).unwrap_or("Missing URL")} }
                }
            }
        }
    }
}

fn display_value(value: &Value) -> String {
    value
        .as_str()
        .filter(|value| !value.is_empty())
        .unwrap_or("Missing")
        .into()
}

/// Dioxus text nodes escape clipboard content rather than inserting HTML.
fn render_steps(steps: &[Value]) -> Element {
    rsx! { ol { for step in steps { li { {display_value(&step["text"])}
        if let Some(children) = step["substeps"].as_array() { if !children.is_empty() { {render_steps(children)} } }
    } } } }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;
    use dioxus_core::NoOpMutations;

    #[test]
    fn provider_selector_renders_accessible_choices_and_escapes_labels() {
        let mut dom = VirtualDom::new_with_props(
            ProviderSelector,
            ProviderSelectorProps {
                provider_choices: vec![
                    ImportProvider {
                        id: "allrecipes".into(),
                        label: "AllRecipes".into(),
                    },
                    ImportProvider {
                        id: "king-arthur-baking".into(),
                        label: "King Arthur Baking".into(),
                    },
                    ImportProvider {
                        id: "test".into(),
                        label: "<script>no</script>".into(),
                    },
                ],
                selected_provider: "king-arthur-baking".into(),
                disabled: true,
                on_change: None,
            },
        );
        dom.rebuild(&mut NoOpMutations);
        let html = dioxus_ssr::render(&dom);
        for expected in [
            "recipe-import-provider",
            "Recipe provider",
            "Automatic",
            "AllRecipes",
            "King Arthur Baking",
            "disabled",
        ] {
            assert!(html.contains(expected), "Missing {expected}: {html}");
        }
        assert_eq!(html.matches("<option").count(), 4);
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn metadata_preview_shows_saved_numbers_source_context_and_unloaded_image_references() {
        let mut candidate: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/recipe-import/v1/basic-visible-recipe/expected.json"
        ))
        .unwrap();
        candidate["source"]["author"] = serde_json::json!("Aster <script>alert(1)</script> Vale");
        candidate["source"]["attribution"] = serde_json::json!("Original by Aster");
        candidate["source"]["suppliedUrl"] = serde_json::json!("https://example.invalid/soup");
        candidate["fields"]["prepTime"]["value"] = serde_json::json!(17);
        candidate["fields"]["cookTime"]["value"] = serde_json::json!(29);
        candidate["fields"]["yield"]["value"]["servings"] = serde_json::json!(7);
        candidate["fields"]["images"] =
            serde_json::json!([{"url":"https://example.invalid/photo.jpg"}]);
        let mut dom = VirtualDom::new_with_props(
            ImportMetadataPreview,
            ImportMetadataPreviewProps {
                candidate,
                keep_context: true,
            },
        );
        dom.rebuild(&mut NoOpMutations);
        let html = dioxus_ssr::render(&dom);
        for expected in [
            "17",
            "29",
            "7",
            "Aster",
            "Original by Aster",
            "https://example.invalid/soup",
            "https://example.invalid/photo.jpg",
            "Commentary",
        ] {
            assert!(html.contains(expected), "Missing {expected} in {html}");
        }
        assert!(!html.contains("<script>"));
        assert!(!html.contains("<img"));
    }
}
