//! Compact local unit completion. Partial queries stay local until confirmed.
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
    let mut editing = use_signal(|| value.clone());
    let mut confirmed = use_signal(|| false);
    // Parent resets (add-row, switching records, save responses) still win when
    // not editing. Partial queries must not enter recipe autosave requests.
    use_effect(use_reactive(
        (&value, &disabled),
        move |(value, disabled)| {
            if disabled || !*open.peek() {
                if value != *editing.peek() {
                    confirmed.set(false);
                }
                editing.set(value);
            }
            if disabled {
                open.set(false);
                active.set(None);
            }
        },
    ));
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
    let mut confirm = move || {
        if !open() {
            return;
        }
        let text = editing();
        let committed = noms::units::completion(&text, active()).map_or_else(
            || {
                if text.trim().is_empty() {
                    String::new()
                } else {
                    text.clone()
                }
            },
            |unit| unit.label.into(),
        );
        editing.set(committed.clone());
        confirmed.set(true);
        oninput.call(committed);
        open.set(false);
        active.set(None);
    };
    let suggestions = noms::units::suggestions(&editing());
    let expanded = !disabled && open() && !suggestions.is_empty();
    let invalid = !open()
        && (confirmed() || !editing().is_empty())
        && noms::units::canonicalize(&editing()).is_err();
    let list_id = format!("{id}-suggestions");
    let error_id = format!("{id}-error");
    let described_by = invalid.then(|| error_id.clone());
    let active_id = active()
        .filter(|i| expanded && *i < suggestions.len())
        .map(|i| format!("{list_id}-{i}"));
    let count = suggestions.len();
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
                aria_invalid: invalid,
                aria_required: true,
                aria_describedby: described_by,
                autocomplete: "off",
                placeholder,
                maxlength,
                disabled,
                value: editing(),
                onfocus: move |_| { open.set(true); active.set(None); },
                onblur: move |_| confirm(),
                oninput: move |event| {
                    editing.set(event.value());
                    open.set(true);
                    active.set(None);
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
                        Key::Enter if open() => {
                            event.prevent_default();
                            confirm();
                        }
                        Key::Escape if expanded => {
                            event.prevent_default();
                            event.stop_propagation();
                            confirm();
                        }
                        Key::Escape | Key::Tab => confirm(),
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
                                title: "{unit.name} — {unit.label}",
                                onmousedown: move |event| event.prevent_default(),
                                onclick: {
                                    let label = unit.label;
                                    move |_| {
                                        editing.set(label.into());
                                        oninput.call(label.into());
                                        open.set(false);
                                        active.set(None);
                                    }
                                },
                                span { class: "unit-autocomplete__name", "{unit.name}" }
                            }
                        }
                    }
                }
            }
            if invalid {
                span { id: error_id, class: "unit-autocomplete__error", role: "alert", "{noms::units::INVALID_UNIT}" }
            }
        }
    }
}
