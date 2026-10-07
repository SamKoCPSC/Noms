//! User-initiated clipboard import opens a validated, editable private draft.
use super::recipe_import_mapping::map_direct_import_candidate;
use crate::{
    api::recipe::create_recipe_draft,
    components::{
        base::{PageIcon, PageIconKind},
        AuthRequired,
    },
    Route,
};
use dioxus::prelude::*;
use serde_json::Value;

const STYLES: Asset = asset!("/assets/recipe-import.css");
const BRIDGE: Asset = asset!("/assets/recipe-import-bridge.mjs");
const PARSER: Asset = asset!("/scripts/recipe-import-parser.mjs");
// JavaScript-only changes must invalidate incremental asset fingerprints.
const _: &[u8] = include_bytes!("../../scripts/recipe-import-parser.mjs");
const _: &[u8] = include_bytes!("../../assets/recipe-import-bridge.mjs");
const CHOOSE_PROVIDER: &str = "choose-provider";

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
        div { class: "recipe-import__field",
            label { r#for: "recipe-import-provider", "Recipe website" span { " · required" } }
            select {
                id: "recipe-import-provider", class: "recipe-import__input neumo-inset", aria_required: "true",
                aria_describedby: "recipe-import-provider-help", value: selected_provider, disabled,
                onchange: move |event| { if let Some(handler) = on_change { handler.call(event.value()); } },
                option { value: CHOOSE_PROVIDER, disabled: true, "Choose a recipe website…" }
                option { value: "", "Automatic (URL or generic parser)" }
                for provider in provider_choices { option { key: "{provider.id}", value: provider.id, "{provider.label}" } }
            }
            p { id: "recipe-import-provider-help", class: "recipe-import__hint", "Choose where you copied the recipe from before importing." }
        }
    }
}

#[component]
pub fn RecipeImport() -> Element {
    rsx! { AuthRequired { ImportForm {} } }
}

/// The shared app shell supplies the main landmark and navigation. Keep the
/// painted heading, source card and instructions local to this page.
#[component]
fn ImportLayout(children: Element) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: STYLES }
        div { class: "container recipe-import",
            nav { class: "recipe-import__breadcrumb", aria_label: "Breadcrumb",
                a { href: "/dashboard", "Your kitchen" } span { aria_hidden: "true", "/" } span { aria_current: "page", "Import recipe" }
            }
            header { class: "recipe-import__header",
                div { class: "recipe-import__identity",
                    PageIcon { kind: PageIconKind::Import }
                    div { h1 { "Bring a recipe home." } p { "Copy a recipe you love. Make it part of your kitchen." } }
                }
                p { class: "recipe-import__privacy", span { aria_hidden: "true", "🔒" } "Private until you choose to share" }
            }
            div { class: "recipe-import__grid",
                section { class: "recipe-import__source neumo-card", aria_label: "Recipe source",
                    p { class: "recipe-import__eyebrow", "01 / THE SOURCE" }
                    h2 { "Paste your recipe" }
                    p { class: "recipe-import__intro", "Start with the page you copied." }
                    {children}
                }
                ImportGuidance {}
            }
        }
    }
}

#[component]
fn ImportGuidance() -> Element {
    rsx! {
        aside { class: "recipe-import__guide", aria_labelledby: "recipe-import-guide-title",
            h2 { id: "recipe-import-guide-title", "For the best import" }
            div { class: "recipe-import__shortcuts neumo-inset",
                table { aria_label: "Keyboard shortcuts for importing a recipe",
                    thead { tr { th { scope: "col", "Action" } th { scope: "col", "Windows / Linux" } th { scope: "col", "Mac" } } }
                    tbody { for (action, letter) in [("Select all", "A"), ("Copy", "C"), ("Paste", "V")] {
                        tr { th { scope: "row", "{action}" } td { kbd { "Ctrl + {letter}" } } td { kbd { "⌘ + {letter}" } } }
                    } }
                }
                p { "Hold Ctrl (or Command ⌘), then press the letter." }
            }
            ol { class: "recipe-import__steps",
                li { strong { "Open one recipe" } p { "Go to the recipe you want to keep. Use a single recipe page, rather than a search result, collection or recipe roundup." } }
                li { strong { "Click inside the page" } p { "Click a plain area near the recipe to focus the page. Avoid links, buttons, search boxes and the address bar." } }
                li { strong { "Select all, then copy" } p { "On the recipe page, press " kbd { "Ctrl + A" } ", then " kbd { "Ctrl + C" } "." br {} "On Mac: " kbd { "⌘ + A" } ", then " kbd { "⌘ + C" } "." } }
                li { strong { "Come back and paste" } p { "Choose the recipe’s website here. Click the Copied recipe page field and press " kbd { "Ctrl + V" } "." br {} "On Mac: " kbd { "⌘ + V" } "." } }
                li { strong { "Import and make it yours" } p { "Click Import recipe. Check the populated fields and Notes in the editor, and correct anything that needs attention before publishing." } }
            }
            div { class: "recipe-import__footnote",
                p { "On a phone or tablet, use your browser’s Select all, Copy and Paste actions." }
                p { "Your recipe starts as a private draft. Images aren’t downloaded during import." }
            }
        }
    }
}

#[component]
fn ImportAction(
    ready: bool,
    captured: bool,
    selected: bool,
    parsing: bool,
    saving: bool,
    on_import: Option<EventHandler<MouseEvent>>,
) -> Element {
    rsx! {
        div { class: "recipe-import__actions",
            p { "Import opens a private draft in the editor." }
            button { r#type: "button", class: "btn btn-primary", disabled: !ready || !captured || !selected || parsing || saving,
                onclick: move |event| { if let Some(handler) = on_import { handler.call(event); } },
                if saving { "Creating draft…" } else if parsing { "Parsing recipe…" } else { "Import recipe" span { aria_hidden: "true", " →" } }
            }
        }
    }
}

/// Only an explicit Import click authorizes draft creation. Paste, typing and
/// source/provider changes cancel that intent as well as any obsolete worker.
#[component]
fn ImportForm() -> Element {
    let mut message = use_signal(|| "Loading clipboard capture…".to_string());
    let mut ready = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut source = use_signal(String::new);
    let mut provider = use_signal(|| CHOOSE_PROVIDER.to_owned());
    let mut provider_choices = use_signal(Vec::<ImportProvider>::new);
    let mut captured = use_signal(|| false);
    let mut parsing = use_signal(|| false);
    let mut saving = use_signal(|| false);
    let mut import_requested = use_signal(|| false);
    let navigator = use_navigator();
    use_drop(move || {
        let _ = document::eval("window.nomsImportCleanup?.(); window.nomsImportCleanup = null;");
    });
    rsx! {
        ImportLayout {
            ProviderSelector { provider_choices: provider_choices(), selected_provider: provider(), disabled: saving() || !ready(),
                on_change: move |value: String| {
                    provider.set(value); error.set(None); parsing.set(false); import_requested.set(false);
                    let detail = serde_json::json!({"sourceUrl":source(),"providerId":provider()});
                    let _ = document::eval(&format!("document.getElementById('recipe-import-paste')?.dispatchEvent(new CustomEvent('noms-source-change', {{detail:{detail}}}));"));
                }
            }
            div { class: "recipe-import__field recipe-import__clipboard",
                label { r#for: "recipe-import-paste", "Copied recipe page" }
                textarea { id: "recipe-import-paste", class: "recipe-import__input neumo-inset", rows: "7", disabled: saving() || !ready(),
                    aria_describedby: "recipe-import-paste-help", placeholder: "Paste the whole recipe page here…\n\nOn the recipe page: select all, copy, then paste here.",
                    onmounted: move |_| {
                        spawn(async move {
                            let script = format!("try {{ const el = document.getElementById('recipe-import-paste'); const {{mountClipboardImport}} = await import({}); const parserUrl = {}; const {{RECIPE_IMPORT_PROVIDERS}} = await import(parserUrl); if (el?.isConnected) window.nomsImportCleanup = mountClipboardImport(el, parserUrl, value => dioxus.send(value.type === 'ready' ? {{...value,providers:RECIPE_IMPORT_PROVIDERS}} : value)); }} catch(e) {{ dioxus.send({{type:'error',message:String(e)}}); }}", serde_json::to_string(&BRIDGE.to_string()).unwrap(), serde_json::to_string(&PARSER.to_string()).unwrap());
                            let mut bridge = document::eval(&script);
                            while let Ok(event) = bridge.recv::<Value>().await {
                                match event["type"].as_str() {
                                    Some("ready") => match serde_json::from_value::<Vec<ImportProvider>>(event["providers"].clone()) {
                                        Ok(choices) if !choices.is_empty() => { provider_choices.set(choices); ready.set(true); message.set("Choose a website and paste your recipe to begin.".into()); }
                                        _ => { error.set(Some("Unable to load recipe websites. Refresh and try again.".into())); }
                                    },
                                    Some("captured") => {
                                        import_requested.set(false); error.set(None); parsing.set(false);
                                        let has_content = event["htmlBytes"].as_u64().unwrap_or(0) > 0 || !event["plainText"].as_str().unwrap_or("").trim().is_empty();
                                        captured.set(has_content);
                                        message.set(if has_content { "Recipe page captured. Ready to import." } else { "Choose a website and paste your recipe to begin." }.into());
                                    }
                                    Some("parsing") => { parsing.set(true); error.set(None); message.set("Parsing your recipe in this browser…".into()); }
                                    Some("result") => {
                                        parsing.set(false);
                                        if !import_requested() || saving() { continue; }
                                        import_requested.set(false);
                                        match map_direct_import_candidate(&event["candidate"]) {
                                            Ok(editor) => {
                                                saving.set(true); message.set("Creating your private draft…".into());
                                                match create_recipe_draft(editor).await {
                                                    Ok(draft) => { navigator.push(Route::RecipeDraftEdit { id: draft.id.to_string() }); }
                                                    Err(problem) => { error.set(Some(format!("Could not create draft: {problem}"))); saving.set(false); message.set("Your pasted recipe is still here. Try importing again.".into()); }
                                                }
                                            }
                                            Err(problem) => { error.set(Some(problem)); message.set("Check your paste and try again. No draft was created.".into()); }
                                        }
                                    }
                                    Some("error") => { parsing.set(false); import_requested.set(false); error.set(Some(event["message"].as_str().unwrap_or("Unable to import this paste.").into())); message.set("Check your paste and try again. No draft was created.".into()); }
                                    _ => {}
                                }
                            }
                        });
                    }
                }
                p { id: "recipe-import-paste-help", class: "recipe-import__hint", "Use " kbd { "Ctrl" } " + " kbd { "V" } " or " kbd { "⌘" } " + " kbd { "V" } " to paste." }
            }
            details { class: "recipe-import__source-details",
                summary { "Source details · optional" }
                div { class: "recipe-import__field",
                    label { r#for: "recipe-import-source", "Original recipe link" }
                    input { id: "recipe-import-source", r#type: "url", class: "recipe-import__input neumo-inset", value: "{source}", disabled: saving() || !ready(),
                        placeholder: "https://…", aria_describedby: "recipe-import-source-help",
                        oninput: move |event| {
                            source.set(event.value()); error.set(None); parsing.set(false); import_requested.set(false);
                            let provider_id = if provider() == CHOOSE_PROVIDER { String::new() } else { provider() };
                            let detail = serde_json::json!({"sourceUrl":source(),"providerId":provider_id});
                            let _ = document::eval(&format!("document.getElementById('recipe-import-paste')?.dispatchEvent(new CustomEvent('noms-source-change', {{detail:{detail}}}));"));
                        }
                    }
                    p { id: "recipe-import-source-help", class: "recipe-import__hint", "Keep the original link for attribution and relative image references. It must match your selected website. Automatic uses this link, or the generic parser when blank. Source details and original wording are kept in the editor’s Notes." }
                }
            }
            p { class: "recipe-import__status", role: "status", aria_live: "polite", "{message}" }
            if let Some(problem) = error() {
                div { class: "recipe-import__error", role: "alert", h3 { "We couldn’t import this recipe" } p { "{problem}" } p { "Copy a single recipe page again, or try its print view. Your paste has not been saved as a recipe." } }
            }
            ImportAction { ready: ready(), captured: captured(), selected: provider() != CHOOSE_PROVIDER, parsing: parsing(), saving: saving(),
                on_import: move |_| {
                    if !ready() || !captured() || provider() == CHOOSE_PROVIDER || parsing() || saving() || import_requested() { return; }
                    import_requested.set(true); parsing.set(true); error.set(None);
                    let detail = serde_json::json!({"sourceUrl":source(),"providerId":provider()});
                    let _ = document::eval(&format!("document.getElementById('recipe-import-paste')?.dispatchEvent(new CustomEvent('noms-preview', {{detail:{detail}}}));"));
                }
            }
            p { class: "recipe-import__local-note", "Parsing happens in your browser. NOMS does not fetch the source page." }
        }
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;
    use dioxus_core::NoOpMutations;

    #[test]
    fn guided_import_layout_has_named_source_and_keyboard_guidance_without_a_preview() {
        let html = dioxus_ssr::render_element(rsx! { ImportLayout { p { "Source controls" } } });
        assert!(html.contains("import-v1"), "{html}");
        assert!(!html.contains("editor-v1"), "{html}");
        for expected in [
            "Bring a recipe home.",
            "Private until you choose to share",
            "Paste your recipe",
            "Recipe source",
            "For the best import",
            "Windows / Linux",
            "Mac",
            "Select all",
            "Copy",
            "Paste",
            "Click inside the page",
            "Images aren’t downloaded",
            "Source controls",
            "recipe-import__grid",
        ] {
            assert!(html.contains(expected), "Missing {expected}: {html}");
        }
        assert!(!html.contains("Recipe preview"));
        assert!(
            !html.contains("<main"),
            "The application already provides the main landmark"
        );
    }

    #[test]
    fn direct_import_action_disables_incomplete_loading_parsing_and_saving_states() {
        for (ready, captured, selected, parsing, saving, disabled, label) in [
            (false, true, true, false, false, true, "Import recipe"),
            (true, false, true, false, false, true, "Import recipe"),
            (true, true, false, false, false, true, "Import recipe"),
            (true, true, true, true, false, true, "Parsing recipe…"),
            (true, true, true, false, true, true, "Creating draft…"),
            (true, true, true, false, false, false, "Import recipe"),
        ] {
            let html = dioxus_ssr::render_element(
                rsx! { ImportAction { ready, captured, selected, parsing, saving, on_import: None } },
            );
            assert_eq!(html.contains("disabled"), disabled, "{html}");
            assert!(html.contains(label), "{html}");
        }
    }

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
            "Recipe website",
            "Choose a recipe website",
            "Automatic",
            "AllRecipes",
            "King Arthur Baking",
            "disabled",
            "required",
            "aria-describedby",
        ] {
            assert!(html.contains(expected), "Missing {expected}: {html}");
        }
        assert_eq!(html.matches("<option").count(), 5);
        assert!(!html.contains("<script>"));
    }
}
