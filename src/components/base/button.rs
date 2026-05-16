use dioxus::prelude::*;

/// Visual variant for the [`Button`] component.
#[derive(Default, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum ButtonVariant {
    /// Filled accent color, primary action.
    #[default]
    Primary,
    /// Outlined with accent border, secondary action.
    Secondary,
    /// Transparent, text-only. For links and minor actions.
    Ghost,
    /// Filled error color, destructive actions.
    Danger,
}

/// Props for the [`Button`] component.
#[derive(Props, Clone, PartialEq)]
pub struct ButtonProps {
    /// Visual style variant.
    #[props(default)]
    pub variant: ButtonVariant,
    /// Whether the button is disabled.
    #[props(default)]
    pub disabled: bool,
    /// Click handler.
    pub onclick: EventHandler<MouseEvent>,
    /// Button content.
    pub children: Element,
}

/// A neumorphic-styled button with multiple visual variants.
#[component]
pub fn Button(props: ButtonProps) -> Element {
    let variant_class = match props.variant {
        ButtonVariant::Primary => "btn-primary",
        ButtonVariant::Secondary => "btn-secondary",
        ButtonVariant::Ghost => "btn-ghost",
        ButtonVariant::Danger => "btn-danger",
    };

    rsx! {
        button {
            class: "btn {variant_class} touch-target",
            disabled: props.disabled,
            onclick: move |evt| props.onclick.call(evt),
            {&props.children}
        }
    }
}
