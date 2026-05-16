use dioxus::prelude::*;

/// Graceful fallback UI when an unhandled error occurs in the router tree.
#[component]
pub fn ErrorFallback(error: ErrorContext) -> Element {
    rsx! {
        div {
            class: "error-fallback",
            h1 { "Something went wrong" }
            p { "Please try refreshing the page." }
            button {
                class: "btn btn-primary touch-target",
                onclick: move |_| {
                    let _ = document::eval("window.location.reload()");
                },
                "Refresh"
            }
        }
    }
}
