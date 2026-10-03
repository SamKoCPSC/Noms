use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub enum CollectionFormError {
    NameRequired,
    NameTooLong,
    DescriptionTooLong,
}

impl std::fmt::Display for CollectionFormError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::NameRequired => "Collection name is required.",
            Self::NameTooLong => "Collection name must be 100 characters or fewer.",
            Self::DescriptionTooLong => "Description must be 500 characters or fewer.",
        })
    }
}

pub fn validate_collection_form(
    name: &str,
    description: &str,
) -> Result<(String, Option<String>), CollectionFormError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CollectionFormError::NameRequired);
    }
    if name.chars().count() > 100 {
        return Err(CollectionFormError::NameTooLong);
    }
    let description = description.trim();
    if description.chars().count() > 500 {
        return Err(CollectionFormError::DescriptionTooLong);
    }
    Ok((
        name.to_string(),
        (!description.is_empty()).then(|| description.to_string()),
    ))
}

#[derive(Props, Clone, PartialEq)]
pub struct CollectionFormProps {
    pub name: Signal<String>,
    pub description: Signal<String>,
    pub submit_label: String,
    pub submitting: bool,
    /// An uncertain creation blocks another write without locking local entries
    /// or Cancel; this is distinct from an actively running submission.
    #[props(default)]
    pub submit_disabled: bool,
    pub error: Option<String>,
    pub onsubmit: EventHandler<()>,
    #[props(default)]
    pub oncancel: Option<EventHandler<()>>,
    #[props(default)]
    pub extra_fields: Option<Element>,
}

// Capture both fields together: updating only the field that emitted an event
// leaves its early-edited sibling vulnerable to a controlled-value rerender.
fn synchronize_visible_collection_fields(
    mut name: Signal<String>,
    mut description: Signal<String>,
    visible_name: String,
    visible_description: String,
) {
    if *name.peek() != visible_name {
        name.set(visible_name);
    }
    if *description.peek() != visible_description {
        description.set(visible_description);
    }
}

#[component]
pub fn CollectionForm(mut props: CollectionFormProps) -> Element {
    rsx! {
        form {
            class: "collection-form",
            aria_busy: props.submitting,
            onmounted: move |_event| {
                #[cfg(target_arch = "wasm32")]
                {
                    use dioxus_web::WebEventExt;
                    use wasm_bindgen::JsCast;

                    // Hydration retains the SSR nodes, but their live values can
                    // differ from the initial Rust signals. Read the mounted form
                    // before a later edit to either field can reset the other.
                    if let Some(form) = _event.data().try_as_web_event() {
                        let name = form.query_selector("[name='collection_name']")
                            .ok().flatten()
                            .and_then(|field| field.dyn_into::<web_sys::HtmlInputElement>().ok());
                        let description = form.query_selector("[name='collection_description']")
                            .ok().flatten()
                            .and_then(|field| field.dyn_into::<web_sys::HtmlTextAreaElement>().ok());
                        if let (Some(name), Some(description)) = (name, description) {
                            synchronize_visible_collection_fields(
                                props.name, props.description, name.value(), description.value(),
                            );
                        }
                    }
                }
            },
            onsubmit: move |event| {
                event.prevent_default();
                if !props.submitting && !props.submit_disabled {
                    // Inputs can change before hydration or via autofill without
                    // an observed input event. Snapshot the visible named fields
                    // before the parent validates and starts a deferred upload.
                    let field_text = |field| match event.get_first(field) {
                        Some(FormValue::Text(value)) => value,
                        _ => String::new(),
                    };
                    synchronize_visible_collection_fields(
                        props.name, props.description,
                        field_text("collection_name"), field_text("collection_description"),
                    );
                    props.onsubmit.call(());
                }
            },
            div { class: "collection-form-layout",
            section { class: "collection-form-panel collection-form-details neumo-raised",
            h2 { "Collection details" }
            p { class: "collection-form-hint", "Give your recipes a place of their own." }
            label {
                class: "collection-form-field",
                span { "Name (required)" }
                input {
                    class: "neumo-inset input",
                    name: "collection_name",
                    value: "{props.name}",
                    maxlength: 100,
                    required: true,
                    disabled: props.submitting,
                    oninput: move |event| props.name.set(event.value()),
                }
            }
            label {
                class: "collection-form-field",
                span { "Description (optional)" }
                textarea {
                    class: "neumo-inset input",
                    name: "collection_description",
                    value: "{props.description}",
                    maxlength: 500,
                    rows: 5,
                    disabled: props.submitting,
                    placeholder: "What belongs in this collection?",
                    oninput: move |event| props.description.set(event.value()),
                }
            }
            p { class: "collection-form-hint", "Names can be up to 100 characters; descriptions up to 500." }
            }
            div { class: "collection-form-options",
            if let Some(extra_fields) = props.extra_fields { {extra_fields} }
            }
            }
            if let Some(error) = props.error {
                p { class: "collection-form-error", role: "alert", "{error}" }
            }
            div { class: "collection-form-actions",
            if let Some(oncancel) = props.oncancel {
                button { class: "btn btn-secondary touch-target", r#type: "button", disabled: props.submitting,
                    onclick: move |_| oncancel.call(()), "Cancel" }
            }
            button { class: "btn btn-primary touch-target", r#type: "submit", disabled: props.submitting || props.submit_disabled,
                if props.submitting { "Saving…" } else { "{props.submit_label}" }
            }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CollectionForm;
    use super::{validate_collection_form, CollectionFormError};
    use dioxus::html::{SerializedFormData, SerializedFormObject, SerializedHtmlEventConverter};
    use dioxus::prelude::*;
    use std::{cell::RefCell, rc::Rc};

    type SubmittedFields = Rc<RefCell<Option<(String, String)>>>;

    #[derive(Props, Clone, PartialEq)]
    struct FormHarnessProps {
        submitted: SubmittedFields,
        submitting: bool,
        submit_disabled: bool,
    }

    #[allow(non_snake_case)]
    fn FormHarness(props: FormHarnessProps) -> Element {
        let name = use_signal(|| "Untitled Collection".to_string());
        let description = use_signal(String::new);
        rsx! { CollectionForm {
            name, description, submit_label: "Save", submitting: props.submitting,
            submit_disabled: props.submit_disabled, error: None,
            oncancel: |_| {},
            onsubmit: move |_| *props.submitted.borrow_mut() = Some((name(), description())),
        } }
    }

    fn submit_without_input_events(
        submitting: bool,
        fields: &[(&str, &str)],
    ) -> (String, Option<(String, String)>) {
        dispatch_form_submit(submitting, false, fields)
    }

    fn dispatch_form_submit(
        submitting: bool,
        submit_disabled: bool,
        fields: &[(&str, &str)],
    ) -> (String, Option<(String, String)>) {
        set_event_converter(Box::new(SerializedHtmlEventConverter));
        let submitted = Rc::new(RefCell::new(None));
        let mut dom = VirtualDom::new_with_props(
            FormHarness,
            FormHarnessProps {
                submitted: submitted.clone(),
                submitting,
                submit_disabled,
            },
        );
        let mutations = dom.rebuild_to_vec();
        let form_id = mutations
            .edits
            .iter()
            .find_map(|edit| match edit {
                dioxus_core::Mutation::NewEventListener { name, id } if *name == "submit" => {
                    Some(*id)
                }
                _ => None,
            })
            .expect("form submit listener");
        let html = dioxus_ssr::render(&dom);
        let data = SerializedFormData::new(
            String::new(),
            fields
                .iter()
                .map(|(key, text)| SerializedFormObject {
                    key: (*key).to_string(),
                    text: Some((*text).to_string()),
                    file: None,
                })
                .collect(),
        );
        dom.runtime().handle_event(
            "submit",
            Event::new(
                Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn std::any::Any>,
                true,
            ),
            form_id,
        );
        let result = submitted.borrow().clone();
        (html, result)
    }

    #[test]
    fn collection_submit_uses_visible_values_even_without_prior_input_events() {
        let (html, submitted) = submit_without_input_events(
            false,
            &[
                ("collection_name", "Typed collection"),
                ("collection_description", "Typed description"),
            ],
        );
        assert_eq!(
            submitted,
            Some((
                "Typed collection".to_string(),
                "Typed description".to_string()
            ))
        );
        assert!(html.contains("name=\"collection_name\""));
        assert!(html.contains("name=\"collection_description\""));
    }

    #[test]
    fn collection_form_reads_early_native_edits_at_mount() {
        let mut dom = VirtualDom::new_with_props(
            FormHarness,
            FormHarnessProps {
                submitted: Rc::new(RefCell::new(None)),
                submitting: false,
                submit_disabled: false,
            },
        );
        let mutations = dom.rebuild_to_vec();
        assert!(
            mutations.edits.iter().any(|edit| matches!(
                edit,
                dioxus_core::Mutation::NewEventListener { name, .. } if name == "mounted"
            )),
            "capture early native field values before subsequent controlled rerenders"
        );
    }

    #[test]
    fn captured_early_values_survive_editing_either_sibling_field() {
        fn harness() -> Element {
            let mut name = use_signal(|| "Untitled Collection".to_string());
            let mut description = use_signal(String::new);
            use_hook(move || {
                super::synchronize_visible_collection_fields(
                    name,
                    description,
                    "Early name 湯麵".into(),
                    "Early description".into(),
                );
                description.set("Later description".into());
                assert_eq!(name(), "Early name 湯麵");
                name.set("Later name".into());
                assert_eq!(description(), "Later description");
            });
            rsx! { CollectionForm {
                name, description, submit_label: "Save", submitting: false, error: None,
                onsubmit: |_| {},
            } }
        }
        let mut dom = VirtualDom::new(harness);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("Later name"));
        assert!(html.contains("Later description"));
    }

    #[test]
    fn collection_submit_cannot_reuse_defaults_when_fields_are_empty_or_missing() {
        let (_, submitted) = submit_without_input_events(
            false,
            &[("collection_name", ""), ("collection_description", "")],
        );
        assert_eq!(submitted, Some((String::new(), String::new())));
        let (_, submitted) = submit_without_input_events(false, &[]);
        assert_eq!(submitted, Some((String::new(), String::new())));
        let (name, description) = submitted.unwrap();
        assert_eq!(
            validate_collection_form(&name, &description),
            Err(CollectionFormError::NameRequired)
        );
    }

    #[test]
    fn collection_submit_does_not_dispatch_a_second_save_while_submitting() {
        let (_, submitted) =
            submit_without_input_events(true, &[("collection_name", "Not a second save")]);
        assert_eq!(submitted, None);
    }

    #[test]
    fn collection_creation_recovery_disables_only_submission_not_local_fields_or_cancel() {
        let source = include_str!("collection_form.rs");
        let component = source.split("#[cfg(test)]").next().unwrap();
        assert!(component.contains("pub submit_disabled: bool"));
        assert!(component.contains("if !props.submitting && !props.submit_disabled"));
        assert!(component.contains("disabled: props.submitting || props.submit_disabled"));
        assert!(component.contains("disabled: props.submitting,"));
    }

    #[test]
    fn unconfirmed_collection_creation_rejects_a_real_form_submit_but_keeps_entries_and_cancel() {
        let (html, submitted) = dispatch_form_submit(
            false,
            true,
            &[("collection_name", "Must not create another collection")],
        );
        assert_eq!(submitted, None);
        assert!(html.contains("value=\"Untitled Collection\""), "{html}");
        let input = html
            .split("name=\"collection_name\"")
            .nth(1)
            .unwrap()
            .split('>')
            .next()
            .unwrap();
        assert!(!input.contains("disabled"), "{html}");
        let cancel = html
            .split(">Cancel</button>")
            .next()
            .unwrap()
            .rsplit("<button")
            .next()
            .unwrap();
        assert!(!cancel.contains("disabled"), "{html}");
        let save = html
            .split(">Save</button>")
            .next()
            .unwrap()
            .rsplit("<button")
            .next()
            .unwrap();
        assert!(save.contains("disabled"), "{html}");
        assert!(!html.contains("Saving…"), "{html}");
    }

    #[test]
    fn collection_form_trims_and_normalizes_valid_fields() {
        assert_eq!(
            validate_collection_form("  Weeknight meals  ", "  Fast dinners  "),
            Ok((
                "Weeknight meals".to_string(),
                Some("Fast dinners".to_string())
            ))
        );
        assert_eq!(
            validate_collection_form("Meals", "   "),
            Ok(("Meals".to_string(), None))
        );
    }

    #[test]
    fn collection_form_rejects_invalid_lengths() {
        assert_eq!(
            validate_collection_form("   ", ""),
            Err(CollectionFormError::NameRequired)
        );
        assert_eq!(
            validate_collection_form(&"x".repeat(101), ""),
            Err(CollectionFormError::NameTooLong)
        );
        assert_eq!(
            validate_collection_form("Meals", &"x".repeat(501)),
            Err(CollectionFormError::DescriptionTooLong)
        );
    }
}
