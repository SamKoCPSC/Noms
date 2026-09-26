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
    pub error: Option<String>,
    pub onsubmit: EventHandler<()>,
    #[props(default)]
    pub oncancel: Option<EventHandler<()>>,
    #[props(default)]
    pub extra_fields: Option<Element>,
}

#[component]
pub fn CollectionForm(mut props: CollectionFormProps) -> Element {
    rsx! {
        form {
            class: "collection-form",
            aria_busy: props.submitting,
            onsubmit: move |event| {
                event.prevent_default();
                if !props.submitting { props.onsubmit.call(()); }
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
            button { class: "btn btn-primary touch-target", r#type: "submit", disabled: props.submitting,
                if props.submitting { "Saving…" } else { "{props.submit_label}" }
            }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{validate_collection_form, CollectionFormError};

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
