use dioxus::prelude::*;

use crate::types::CollectionVisibility;

const PRIVATE_EXPLANATION: &str = "Private: only you can access it.";
const UNLISTED_EXPLANATION: &str = "Unlisted: anyone with the link can access shareable contents.";
const PUBLIC_EXPLANATION: &str = "Public: anyone can access it and it may appear in discovery.";

fn visibility_label(visibility: CollectionVisibility) -> &'static str {
    match visibility {
        CollectionVisibility::Private => "Private",
        CollectionVisibility::Unlisted => "Unlisted",
        CollectionVisibility::Public => "Public",
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct CollectionVisibilityFieldProps {
    pub visibility: CollectionVisibility,
    pub onchange: EventHandler<CollectionVisibility>,
    #[props(default)]
    pub disabled: bool,
}

#[component]
pub fn CollectionVisibilityField(props: CollectionVisibilityFieldProps) -> Element {
    // Mount and normal changes must use the same validation path.
    // Native choices made before hydration have not emitted a Rust event yet.
    let update_from_visible_value = move |value: String| {
        if props.disabled {
            return;
        }
        if let Ok(visibility) = value.parse::<CollectionVisibility>() {
            if visibility != props.visibility {
                props.onchange.call(visibility);
            }
        }
    };
    let explanation = match props.visibility {
        CollectionVisibility::Private => PRIVATE_EXPLANATION,
        CollectionVisibility::Unlisted => UNLISTED_EXPLANATION,
        CollectionVisibility::Public => PUBLIC_EXPLANATION,
    };
    rsx! {
        label { display: "flex", flex_direction: "column", gap: "var(--space-xs)",
            span { font_weight: "600", "Visibility" }
            select {
                class: "neumo-inset input",
                value: props.visibility.as_str(),
                disabled: props.disabled,
                onmounted: move |_event| {
                    #[cfg(target_arch = "wasm32")]
                    {
                        use dioxus_web::WebEventExt;
                        use wasm_bindgen::JsCast;

                        // Hydration preserves the live select node. Capture its
                        // value before later rerenders or Save can use stale props.
                        let Some(select) = _event.data().try_as_web_event()
                            .and_then(|element| element.dyn_into::<web_sys::HtmlSelectElement>().ok())
                        else { return; };
                        update_from_visible_value(select.value());
                    }
                },
                onchange: move |event| update_from_visible_value(event.value()),
                // A select's value attribute does not select its SSR option.
                option { value: "private", selected: props.visibility == CollectionVisibility::Private, "Private" }
                option { value: "unlisted", selected: props.visibility == CollectionVisibility::Unlisted, "Unlisted" }
                option { value: "public", selected: props.visibility == CollectionVisibility::Public, "Public" }
            }
            small { color: "var(--text-secondary)", "{explanation}" }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct CollectionVisibilityBadgeProps {
    pub visibility: CollectionVisibility,
}

#[component]
pub fn CollectionVisibilityBadge(props: CollectionVisibilityBadgeProps) -> Element {
    let label = visibility_label(props.visibility);
    rsx! {
        span {
            display: "inline-flex",
            align_items: "center",
            width: "fit-content",
            padding: "var(--space-xs) var(--space-sm)",
            border: "1px solid var(--glass-border)",
            border_radius: "var(--radius-full)",
            background: "var(--glass-fill)",
            color: "var(--text-secondary)",
            font_size: "12px",
            font_weight: "600",
            aria_label: "Collection visibility: {label}",
            "{label}"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::html::{SerializedFormData, SerializedHtmlEventConverter};
    use std::{cell::RefCell, rc::Rc};

    #[derive(Props, Clone, PartialEq)]
    struct VisibilityHarnessProps {
        initial: CollectionVisibility,
        disabled: bool,
        changes: Rc<RefCell<Vec<CollectionVisibility>>>,
    }

    #[allow(non_snake_case)]
    fn VisibilityHarness(props: VisibilityHarnessProps) -> Element {
        let mut visibility = use_signal(|| props.initial);
        rsx! { CollectionVisibilityField {
            visibility: visibility(), disabled: props.disabled,
            onchange: move |selected| {
                props.changes.borrow_mut().push(selected);
                visibility.set(selected);
            },
        } }
    }

    fn visibility_dom(
        initial: CollectionVisibility,
        disabled: bool,
    ) -> (VirtualDom, Rc<RefCell<Vec<CollectionVisibility>>>) {
        let changes = Rc::new(RefCell::new(Vec::new()));
        let dom = VirtualDom::new_with_props(
            VisibilityHarness,
            VisibilityHarnessProps {
                initial,
                disabled,
                changes: changes.clone(),
            },
        );
        (dom, changes)
    }

    #[test]
    fn visibility_ssr_marks_only_the_actual_option_selected() {
        for visibility in [
            CollectionVisibility::Private,
            CollectionVisibility::Unlisted,
            CollectionVisibility::Public,
        ] {
            let (mut dom, _) = visibility_dom(visibility, false);
            dom.rebuild_in_place();
            let html = dioxus_ssr::render(&dom);
            let selected_options: Vec<_> = html
                .split("<option")
                .skip(1)
                .map(|option| option.split('>').next().unwrap())
                .filter(|option| option.contains("selected"))
                .collect();
            assert_eq!(
                selected_options.len(),
                1,
                "SSR must select exactly one option: {html}"
            );
            assert!(
                selected_options[0].contains(&format!("value=\"{}\"", visibility.as_str())),
                "{html}"
            );
        }
    }

    #[test]
    fn visibility_registers_live_selection_capture_at_mount() {
        let (mut dom, _) = visibility_dom(CollectionVisibility::Private, false);
        let mutations = dom.rebuild_to_vec();
        assert!(
            mutations.edits.iter().any(|edit| matches!(edit,
                dioxus_core::Mutation::NewEventListener { name, .. } if *name == "mounted"
            )),
            "sharing must capture a native selection made before hydration"
        );
    }

    #[test]
    fn visibility_change_updates_explanation_and_ignores_invalid_or_disabled_choices() {
        set_event_converter(Box::new(SerializedHtmlEventConverter));
        for disabled in [false, true] {
            let (mut dom, changes) = visibility_dom(CollectionVisibility::Private, disabled);
            let mutations = dom.rebuild_to_vec();
            let select_id = mutations
                .edits
                .iter()
                .find_map(|edit| match edit {
                    dioxus_core::Mutation::NewEventListener { name, id } if *name == "change" => {
                        Some(*id)
                    }
                    _ => None,
                })
                .expect("visibility change listener");
            for value in ["unlisted", "unlisted", "public", "invalid"] {
                dom.runtime().handle_event(
                    "change",
                    Event::new(
                        Rc::new(PlatformEventData::new(Box::new(SerializedFormData::new(
                            value.to_string(),
                            Vec::new(),
                        )))) as Rc<dyn std::any::Any>,
                        true,
                    ),
                    select_id,
                );
                dom.render_immediate_to_vec();
            }
            let html = dioxus_ssr::render(&dom);
            if disabled {
                assert!(
                    changes.borrow().is_empty(),
                    "disabled controls must not change sharing"
                );
                assert!(html.contains(PRIVATE_EXPLANATION), "{html}");
            } else {
                assert_eq!(
                    *changes.borrow(),
                    vec![CollectionVisibility::Unlisted, CollectionVisibility::Public]
                );
                assert!(html.contains(PUBLIC_EXPLANATION), "{html}");
            }
        }
    }

    #[test]
    fn visibility_field_and_badge_explain_all_supported_states() {
        let source = include_str!("collection_visibility.rs");
        assert!(source.contains("CollectionVisibilityField"));
        assert!(source.contains("CollectionVisibilityBadge"));
        assert!(source.contains("Private: only you can access it."));
        assert!(source.contains("Unlisted: anyone with the link"));
        assert!(source.contains("Public: anyone can access it"));
        assert!(source.contains("select"));
    }
}
