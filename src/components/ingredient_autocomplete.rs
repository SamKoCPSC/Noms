//! Optional ingredient suggestions. Typing is always valid without a lookup.
use dioxus::prelude::*;
use uuid::Uuid;

#[component]
pub fn IngredientAutocomplete(
    #[props(default)] index: usize,
    value: String,
    oninput: EventHandler<(String, Option<Uuid>)>,
    #[props(default)] id: Option<String>,
    #[props(default)] accessible_name: Option<String>,
    #[props(default = "Ingredient name".into())] placeholder: String,
    #[props(default)] maxlength: Option<i64>,
    #[props(default)] required: bool,
    #[props(default)] disabled: bool,
) -> Element {
    let mut query = use_signal(String::new);
    let mut open = use_signal(|| false);
    let mut active = use_signal(|| None::<usize>);
    let vocabulary = super::ingredient_vocabulary::state();
    let results = use_resource(move || async move {
        let text = query();
        if vocabulary.read().index.is_some() || text.trim().chars().count() < 2 {
            return (text, Some(Vec::new()));
        }
        #[cfg(target_arch = "wasm32")]
        {
            let (send, receive) = futures_channel::oneshot::channel();
            let _timer = gloo_timers::callback::Timeout::new(200, move || {
                let _ = send.send(());
            });
            let _ = receive.await;
        }
        if vocabulary.peek().index.is_some() {
            return (text, Some(Vec::new()));
        }
        let response = crate::api::vocabulary::ingredient_suggestions(text.clone())
            .await
            .ok();
        (text, response)
    });
    // Never display a response for an older value, even before cancellation runs.
    let result = results.read();
    let search_index = vocabulary.read().index.clone();
    let suggestions = search_index
        .as_ref()
        .map(|i| i.search(&value))
        .unwrap_or_else(|| {
            result
                .as_ref()
                .filter(|(q, _)| q == &value)
                .and_then(|(_, rows)| rows.clone())
                .unwrap_or_default()
        });
    let unavailable = search_index.is_none()
        && result
            .as_ref()
            .is_some_and(|(q, rows)| q == &value && rows.is_none());
    let expanded = !disabled && open() && !suggestions.is_empty();
    let list_id = id.as_ref().map_or_else(
        || format!("ingredient-suggestions-{index}"),
        |id| format!("{id}-suggestions"),
    );
    let accessible_name =
        accessible_name.unwrap_or_else(|| format!("Ingredient {} name", index + 1));
    let active_id = active()
        .filter(|i| expanded && *i < suggestions.len())
        .map(|i| format!("{list_id}-{i}"));
    let keyboard_rows = suggestions.clone();
    rsx! {
        document::Stylesheet { href: asset!("/assets/ingredient-autocomplete.css") }
        div { class: "ingredient-autocomplete",
            input {
                id,
                class: "neumo-inset input input--shared",
                r#type: "text",
                aria_label: accessible_name,
                role: "combobox",
                aria_autocomplete: "list",
                aria_expanded: expanded,
                aria_controls: list_id.clone(),
                aria_activedescendant: active_id,
                autocomplete: "off",
                placeholder,
                maxlength,
                required,
                disabled,
                value: value.clone(),
                onfocus: move |_| {
                    open.set(true);
                    super::ingredient_vocabulary::on_focus(vocabulary);
                },
                onblur: move |_| { open.set(false); active.set(None); },
                oninput: move |event| {
                    let text = event.value();
                    query.set(text.clone());
                    open.set(true);
                    active.set(None);
                    // Every text edit clears the old recognition reference.
                    oninput.call((text, None));
                },
                onkeydown: move |event| {
                    let count = keyboard_rows.len();
                    match event.key() {
                        Key::ArrowDown if count > 0 => {
                            event.prevent_default();
                            open.set(true);
                            active.set(Some(active().map_or(0, |i| (i + 1) % count)));
                        }
                        Key::ArrowUp if count > 0 => {
                            event.prevent_default();
                            open.set(true);
                            active.set(Some(active().map_or(count - 1, |i| (i + count - 1) % count)));
                        }
                        Key::Enter if open() => {
                            if let Some(row) = active().and_then(|i| keyboard_rows.get(i)) {
                                event.prevent_default();
                                oninput.call((row.label.clone(), Some(row.id)));
                                open.set(false);
                                active.set(None);
                            }
                        }
                        Key::Escape if expanded => {
                            event.prevent_default();
                            event.stop_propagation();
                            open.set(false);
                            active.set(None);
                        }
                        Key::Escape | Key::Tab => { open.set(false); active.set(None); }
                        _ => {}
                    }
                },
            }
            if expanded {
                crate::components::base::autocomplete_popup::AutocompleteList {
                    id: list_id.clone(), accessible_name: "Ingredient suggestions",
                    query: value.clone(), active: active(),
                    for (position, row) in suggestions.iter().enumerate() {
                        li { role: "presentation",
                            button {
                                id: "{list_id}-{position}",
                                key: "{row.id}",
                                r#type: "button",
                                role: "option",
                                aria_selected: active() == Some(position),
                                tabindex: "-1",
                                onpointerdown: move |event| event.prevent_default(),
                                onmousedown: move |event| event.prevent_default(),
                                onclick: {
                                    let row = row.clone();
                                    move |_| {
                                        oninput.call((row.label.clone(), Some(row.id)));
                                        open.set(false);
                                        active.set(None);
                                    }
                                },
                                strong { "{row.label}" }
                            }
                        }
                    }
                }
            }
            if !disabled && open() && unavailable {
                span { class: "ingredient-autocomplete__hint", role: "status", "Suggestions unavailable. You can still enter any ingredient." }
            }
        }
    }
}
