//! Precise unit suggestions are bundled locally; authored text remains optional.
use dioxus::prelude::*;

#[component]
pub fn UnitAutocomplete(
    id: String,
    value: String,
    oninput: EventHandler<String>,
    #[props(default = "Unit".into())] accessible_name: String,
    #[props(default = "Unit".into())] placeholder: String,
    #[props(default)] maxlength: Option<i64>,
    #[props(default)] disabled: bool,
    #[props(default = true)] align_end: bool,
) -> Element {
    let mut open = use_signal(|| false);
    let mut active = use_signal(|| None::<usize>);
    let suggestions = noms::units::suggestions(&value);
    let expanded = !disabled && open() && !suggestions.is_empty();
    let list_id = format!("{id}-suggestions");
    let active_id = active()
        .filter(|i| expanded && *i < suggestions.len())
        .map(|i| format!("{list_id}-{i}"));
    let keyboard_rows = suggestions.clone();
    rsx! {
        document::Stylesheet { href: asset!("/assets/ingredient-autocomplete.css") }
        div {
            class: if align_end { "ingredient-autocomplete unit-autocomplete unit-autocomplete--end" } else { "ingredient-autocomplete unit-autocomplete" },
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
                            if let Some(unit) = active().and_then(|i| keyboard_rows.get(i)) {
                                event.prevent_default();
                                oninput.call(unit.label.into());
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
                ul { id: list_id.clone(), class: "ingredient-autocomplete__list neumo-raised", role: "listbox", aria_label: "Unit suggestions",
                    for (position, unit) in suggestions.iter().enumerate() {
                        li { role: "presentation",
                            button {
                                id: "{list_id}-{position}",
                                key: "{unit.label}",
                                r#type: "button",
                                role: "option",
                                aria_label: unit.name,
                                aria_selected: active() == Some(position),
                                tabindex: "-1",
                                onmousedown: move |event| event.prevent_default(),
                                onclick: {
                                    let label = unit.label;
                                    move |_| {
                                        oninput.call(label.into());
                                        open.set(false);
                                        active.set(None);
                                    }
                                },
                                strong { "{unit.label}" }
                                span { class: "unit-autocomplete__name", "{unit.name}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
