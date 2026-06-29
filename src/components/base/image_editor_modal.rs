//! Image editor modal: displays a selected image in a modal overlay.
//!
//! Used by `ImageUpload` to open the image editor for a pending image after
//! the user selects a file.

use dioxus::prelude::*;

use crate::components::base::{Button, ButtonVariant};

/// Props for [`ImageEditorModal`].
#[derive(Props, Clone, PartialEq)]
pub struct ImageEditorModalProps {
    /// The image URL to display. `None` hides the modal.
    pub image_url: Signal<Option<String>>,
    /// Called to close the modal (sets the parent's signal to `None`).
    pub on_close: EventHandler<()>,
}

/// Modal that opens the image editor for a selected image.
#[component]
pub fn ImageEditorModal(props: ImageEditorModalProps) -> Element {
    let image_url = props.image_url;
    let on_close = props.on_close;

    if image_url().is_none() {
        return VNode::empty();
    }

    rsx! {
        div {
            class: "modal-overlay",
            onclick: move |_| {
                on_close.call(());
            },
            div {
                class: "modal-card modal-card--editor",
                onclick: move |evt| evt.stop_propagation(),
                // Close button
                div {
                    position: "absolute",
                    top: "var(--space-sm)",
                    right: "var(--space-sm)",
                    z_index: "1",
                    Button {
                        variant: ButtonVariant::Ghost,
                        onclick: move |_| {
                            on_close.call(());
                        },
                        "✕"
                    }
                }
                // Image
                if let Some(ref url) = image_url() {
                    img {
                        class: "image-editor__image",
                        src: url,
                        alt: "Image editor",
                    }
                }
            }
        }
    }
}
