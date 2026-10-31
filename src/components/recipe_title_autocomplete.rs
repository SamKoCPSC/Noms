//! Live query completions: selection submits title text, never a recipe identity.
use dioxus::prelude::*;

#[component]
pub(crate) fn RecipeTitleAutocomplete(
    id: String,
    value: String,
    oninput: EventHandler<String>,
    onsubmit: EventHandler<String>,
    accessible_name: String,
    #[props(default = "Search recipes…".into())] placeholder: String,
    #[props(default = "input input--shared".into())] input_class: String,
    #[props(default)] public_only: bool,
) -> Element {
    let mut open = use_signal(|| false);
    let mut active = use_signal(|| None::<usize>);
    let auth = crate::auth::context::use_auth();
    let viewer = auth.current_user_id.filter(|_| auth.is_authenticated);
    let results = use_resource(use_reactive(
        (&value, &public_only, &viewer),
        move |(text, public_only, viewer)| {
            let focused = open();
            async move {
                if !focused || crate::search::validate_autocomplete_text(&text).is_err() {
                    return (text, public_only, viewer, Ok(Vec::new()));
                }
                #[cfg(target_arch = "wasm32")]
                gloo_timers::future::TimeoutFuture::new(200).await;
                let response = crate::api::search::suggest_recipe_titles(text.clone(), public_only)
                    .await
                    .map_err(|_| ());
                (text, public_only, viewer, response)
            }
        },
    ));
    // Late responses cannot appear for a new query, audience or signed-in account.
    let response = results.read();
    let current = response.as_ref().filter(|(query, public, user, _)| {
        query == &value && *public == public_only && *user == viewer
    });
    let suggestions = current
        .and_then(|(_, _, _, rows)| rows.as_ref().ok())
        .cloned()
        .unwrap_or_default();
    let unavailable = current.is_some_and(|(_, _, _, rows)| rows.is_err());
    let expanded = open() && !suggestions.is_empty();
    let list_id = format!("{id}-suggestions");
    let active_id = active()
        .filter(|i| expanded && *i < suggestions.len())
        .map(|i| format!("{list_id}-{i}"));
    let count = suggestions.len();
    let keyboard_rows = suggestions.clone();
    let keyboard_value = value.clone();
    rsx! {
        document::Stylesheet { href: asset!("/assets/ingredient-autocomplete.css") }
        div { class: "ingredient-autocomplete recipe-title-autocomplete",
            input {
                id, name: "q", r#type: "search", class: input_class,
                aria_label: accessible_name, placeholder, maxlength: "256", autocomplete: "off",
                role: "combobox", aria_autocomplete: "list", aria_expanded: expanded,
                aria_controls: list_id.clone(), aria_activedescendant: active_id,
                value: value.clone(),
                onfocus: move |_| { open.set(true); active.set(None); },
                onblur: move |_| { open.set(false); active.set(None); },
                oninput: move |event| { active.set(None); open.set(true); oninput.call(event.value()); },
                onkeydown: move |event| {
                    match event.key() {
                        Key::ArrowDown if count > 0 => {
                            event.prevent_default(); if !open() { open.set(true); }
                            active.set(Some(active().map_or(0, |i| (i + 1) % count)));
                        }
                        Key::ArrowUp if count > 0 => {
                            event.prevent_default(); if !open() { open.set(true); }
                            active.set(Some(active().map_or(count - 1, |i| (i + count - 1) % count)));
                        }
                        Key::Enter => {
                            event.prevent_default();
                            let text = if expanded { active().and_then(|i| keyboard_rows.get(i)).cloned() } else { None }
                                .unwrap_or_else(|| keyboard_value.clone());
                            open.set(false); active.set(None); onsubmit.call(text);
                        }
                        Key::Escape if open() => {
                            event.prevent_default(); event.stop_propagation(); open.set(false); active.set(None);
                        }
                        Key::Tab => { open.set(false); active.set(None); }
                        _ => {}
                    }
                },
            }
            if expanded {
                crate::components::base::autocomplete_popup::AutocompleteList {
                    id: list_id.clone(), accessible_name: "Recipe title suggestions", query: value.clone(), active: active(),
                    for (position, title) in suggestions.into_iter().enumerate() {
                        li { role: "presentation",
                            button {
                                id: "{list_id}-{position}", key: title.clone(), r#type: "button", role: "option",
                                aria_selected: active() == Some(position), tabindex: "-1",
                                onpointerdown: move |event| event.prevent_default(),
                                onmousedown: move |event| event.prevent_default(),
                                onclick: { let title = title.clone(); move |_| { open.set(false); active.set(None); onsubmit.call(title.clone()); } },
                                strong { "{title}" }
                            }
                        }
                    }
                }
            }
            if open() && unavailable {
                span { class: "ingredient-autocomplete__hint", role: "status", "Suggestions unavailable. Press Enter to search." }
            }
        }
    }
}
