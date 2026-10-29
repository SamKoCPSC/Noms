//! Optional equipment completion; only an explicit choice changes the name.
use dioxus::prelude::*;

#[component]
pub fn EquipmentAutocomplete(
    id: String,
    value: String,
    oninput: EventHandler<String>,
    accessible_name: String,
    #[props(default = "e.g. mixing bowl".into())] placeholder: String,
    #[props(default)] disabled: bool,
) -> Element {
    let mut open = use_signal(|| false);
    let mut active = use_signal(|| None::<usize>);
    // Row removal, a draft reload or external value changes invalidate the
    // highlighted suggestion. Text is always controlled by the recipe editor.
    use_effect(use_reactive((&value, &disabled), move |(_, disabled)| {
        active.set(None);
        if disabled {
            open.set(false);
        }
    }));
    #[cfg(target_arch = "wasm32")]
    use_effect(use_reactive((&id,), move |(id,)| {
        if open() {
            if let Some(position) = active() {
                if let Some(element) = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.get_element_by_id(&format!("{id}-suggestions-{position}")))
                {
                    let options = web_sys::ScrollIntoViewOptions::new();
                    options.set_block(web_sys::ScrollLogicalPosition::Nearest);
                    options.set_inline(web_sys::ScrollLogicalPosition::Nearest);
                    element.scroll_into_view_with_scroll_into_view_options(&options);
                }
            }
        }
    }));
    let suggestions = noms::equipment::suggestions(&value);
    let expanded = !disabled && open() && !suggestions.is_empty();
    let list_id = format!("{id}-suggestions");
    let active_id = active()
        .filter(|i| expanded && *i < suggestions.len())
        .map(|i| format!("{list_id}-{i}"));
    let count = suggestions.len();
    let keyboard_rows = suggestions.clone();
    rsx! {
        document::Stylesheet { href: asset!("/assets/ingredient-autocomplete.css") }
        div { class: "ingredient-autocomplete equipment-autocomplete",
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
                disabled,
                value,
                onfocus: move |_| { open.set(true); active.set(None); },
                onblur: move |_| { open.set(false); active.set(None); },
                oninput: move |event| {
                    open.set(true);
                    active.set(None);
                    oninput.call(event.value());
                },
                onkeydown: move |event| {
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
                        Key::Enter if expanded => {
                            if let Some(row) = active().and_then(|i| keyboard_rows.get(i)) {
                                event.prevent_default();
                                oninput.call((*row).into());
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
                ul {
                    id: list_id.clone(),
                    class: "ingredient-autocomplete__list neumo-raised",
                    role: "listbox",
                    aria_label: "Equipment suggestions",
                    for (position, label) in suggestions.into_iter().enumerate() {
                        li { role: "presentation",
                            button {
                                id: "{list_id}-{position}",
                                key: label,
                                r#type: "button",
                                role: "option",
                                aria_selected: active() == Some(position),
                                tabindex: "-1",
                                onpointerdown: move |event| event.prevent_default(),
                                onmousedown: move |event| event.prevent_default(),
                                onclick: move |_| {
                                    oninput.call(label.into());
                                    open.set(false);
                                    active.set(None);
                                },
                                strong { "{label}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
