//! A themed menu keeps recipe sharing choices inside the app on touch devices.
use dioxus::prelude::*;

const OPTIONS: [(&str, &str, &str); 3] = [
    ("private", "Private", "Only you can see this recipe"),
    ("unlisted", "Unlisted", "Anyone with the link can view"),
    ("public", "Public", "Appears in Explore and search"),
];

#[component]
pub fn RecipeVisibilityField(
    id: String,
    visibility: String,
    onchange: EventHandler<String>,
) -> Element {
    let trigger_id = format!("{id}-trigger");
    let panel_id = format!("{id}-panel");
    let label_id = format!("{id}-label");
    let value_id = format!("{id}-value");
    let description_id = format!("{id}-description");
    let (_, label, description) = OPTIONS
        .iter()
        .find(|(value, _, _)| *value == visibility)
        .copied()
        .unwrap_or(OPTIONS[0]);
    #[cfg(target_arch = "wasm32")]
    {
        let cleanup_id = id.clone();
        use_drop(move || super::account_menu::dispose_account_menu(&cleanup_id));
    }
    let mount_id = id.clone();
    rsx! {
        document::Stylesheet { href: asset!("/assets/ingredient-autocomplete.css") }
        document::Stylesheet { href: asset!("/assets/recipe-visibility.css") }
        div {
            id, class: "recipe-visibility",
            onmounted: move |_| {
                #[cfg(target_arch = "wasm32")]
                super::account_menu::mount_menu(&mount_id, "[data-visibility-trigger]", "[data-visibility-panel]");
                #[cfg(not(target_arch = "wasm32"))]
                let _ = &mount_id;
            },
            label { id: label_id.clone(), r#for: trigger_id.clone(), class: "recipe-visibility__label", "Visibility when published" }
            button {
                id: trigger_id, r#type: "button", class: "neumo-inset input recipe-visibility__trigger",
                "data-visibility-trigger": "",
                aria_haspopup: "menu", aria_expanded: "false", aria_controls: panel_id.clone(),
                aria_describedby: description_id.clone(),
                span { id: value_id, "{label}" }
                svg { width: "20", height: "20", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", "aria-hidden": "true",
                    path { d: "m6 9 6 6 6-6", stroke_linecap: "round", stroke_linejoin: "round" }
                }
            }
            span { id: description_id, class: "recipe-visibility__description", "{description}" }
            div {
                id: panel_id, hidden: true, role: "menu", aria_labelledby: label_id,
                "data-visibility-panel": "",
                "data-menu-width": "trigger",
                class: "ingredient-autocomplete__list neumo-raised recipe-visibility__menu",
                for (value, title, _) in OPTIONS {
                    button {
                        key: "{value}", r#type: "button", role: "menuitemradio", tabindex: "-1",
                        class: "recipe-visibility__option",
                        aria_checked: (visibility == value).to_string(),
                        onclick: move |_| onchange.call(value.to_string()),
                        strong { "{title}" }
                        span { class: "recipe-visibility__check", aria_hidden: "true",
                            if visibility == value { "✓" }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::html::{SerializedHtmlEventConverter, SerializedMouseData};
    use std::{cell::RefCell, rc::Rc};

    #[component]
    fn Harness(initial: String, changes: Rc<RefCell<Vec<String>>>) -> Element {
        let mut visibility = use_signal(|| initial);
        rsx! {
            RecipeVisibilityField {
                id: "test-visibility", visibility: visibility(),
                onchange: move |value: String| {
                    changes.borrow_mut().push(value.clone());
                    visibility.set(value);
                },
            }
        }
    }

    fn harness(initial: &str, changes: Rc<RefCell<Vec<String>>>) -> VirtualDom {
        VirtualDom::new_with_props(
            Harness,
            HarnessProps {
                initial: initial.into(),
                changes,
            },
        )
    }

    #[test]
    fn recipe_visibility_ssr_exposes_one_checked_choice_and_the_current_summary() {
        for (value, label, description) in OPTIONS {
            let mut dom = harness(value, Rc::default());
            dom.rebuild_in_place();
            let html = dioxus_ssr::render(&dom);
            assert!(
                !html.contains("<select"),
                "Mobile must not open a native picker"
            );
            assert_eq!(html.matches("role=\"menuitemradio\"").count(), 3);
            assert_eq!(html.matches("aria-checked=\"true\"").count(), 1);
            let summary = html
                .split("id=\"test-visibility-value\"")
                .nth(1)
                .unwrap()
                .split("</button>")
                .next()
                .unwrap();
            assert!(summary.contains(label), "{html}");
            assert!(html.contains(description), "{html}");
            assert!(
                html.contains("aria-describedby=\"test-visibility-description\""),
                "{html}"
            );
        }
    }

    #[test]
    fn recipe_visibility_selection_updates_the_parent_and_checked_option() {
        set_event_converter(Box::new(SerializedHtmlEventConverter));
        let changes = Rc::new(RefCell::new(Vec::new()));
        let mut dom = harness("private", changes.clone());
        let mutations = dom.rebuild_to_vec();
        let buttons: Vec<_> = mutations
            .edits
            .iter()
            .filter_map(|edit| match edit {
                dioxus_core::Mutation::NewEventListener { name, id } if name == "click" => {
                    Some(*id)
                }
                _ => None,
            })
            .collect();
        assert_eq!(buttons.len(), 3);
        for index in [1, 2, 0] {
            dom.runtime().handle_event(
                "click",
                Event::new(
                    Rc::new(PlatformEventData::new(Box::new(
                        SerializedMouseData::default(),
                    ))) as Rc<dyn std::any::Any>,
                    true,
                ),
                buttons[index],
            );
            dom.render_immediate_to_vec();
            assert_eq!(changes.borrow().last().unwrap(), OPTIONS[index].0);
            let html = dioxus_ssr::render(&dom);
            let checked = html
                .split("<button")
                .skip(1)
                .find(|button| button.contains("aria-checked=\"true\""))
                .unwrap();
            assert!(checked.contains(OPTIONS[index].1), "{html}");
            assert_eq!(html.matches("aria-checked=\"true\"").count(), 1);
        }
    }
}
