use dioxus::prelude::*;

/// Props for the [`Input`] component.
#[derive(Props, Clone, PartialEq)]
pub struct InputProps {
    /// Current value.
    #[props(default)]
    pub value: String,
    /// Placeholder text.
    #[props(default)]
    pub placeholder: String,
    /// Input type (text, email, password, etc.).
    #[props(default = "text".to_string())]
    pub input_type: String,
    /// Change handler.
    pub oninput: EventHandler<String>,
}

/// A neumorphic-inset text input.
#[component]
pub fn Input(props: InputProps) -> Element {
    rsx! {
        input {
            class: "neumo-inset input input--shared",
            r#type: "{props.input_type}",
            value: "{props.value}",
            placeholder: "{props.placeholder}",
            oninput: move |evt| props.oninput.call(evt.value()),
        }
    }
}
