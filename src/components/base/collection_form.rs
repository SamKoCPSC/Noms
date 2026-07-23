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
    pub extra_fields: Option<Element>,
}

#[component]
pub fn CollectionForm(mut props: CollectionFormProps) -> Element {
    rsx! {
        form {
            onsubmit: move |event| {
                event.prevent_default();
                props.onsubmit.call(());
            },
            display: "flex",
            flex_direction: "column",
            gap: "var(--space-lg)",
            label {
                display: "flex", flex_direction: "column", gap: "var(--space-xs)",
                span { "Name" }
                input {
                    class: "neumo-inset input",
                    value: "{props.name}",
                    maxlength: 100,
                    required: true,
                    oninput: move |event| props.name.set(event.value()),
                }
            }
            label {
                display: "flex", flex_direction: "column", gap: "var(--space-xs)",
                span { "Description (optional)" }
                textarea {
                    class: "neumo-inset input",
                    value: "{props.description}",
                    maxlength: 500,
                    rows: 5,
                    oninput: move |event| props.description.set(event.value()),
                }
            }
            if let Some(extra_fields) = props.extra_fields { {extra_fields} }
            if let Some(error) = props.error {
                p { color: "var(--error)", role: "alert", "{error}" }
            }
            button { class: "btn btn-primary touch-target", r#type: "submit", disabled: props.submitting,
                if props.submitting { "Saving…" } else { "{props.submit_label}" }
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
