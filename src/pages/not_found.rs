use dioxus::prelude::*;

use crate::Route;

/// 404 page shown when no route matches the current URL.
#[component]
pub fn NotFound(segments: Vec<String>) -> Element {
    let path = if segments.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", segments.join("/"))
    };

    rsx! {
        div { class: "not-found container",

            // Large branded emoji/icon
            div {
                font_size: "72px",
                margin_bottom: "var(--space-md)",
                "🍽️"
            }

            h1 { "404" }

            h2 { "Page not found" }

            p {
                font_size: "16px",
                margin_bottom: "var(--space-xs)",
                max_width: "420px",
                "The page you're looking for doesn't exist or has been moved."
            }

            // Show the unmatched path in a subtle code block
            if !segments.is_empty() {
                p {
                    margin_bottom: "var(--space-xl)",
                    font_size: "14px",
                    code { "{path}" }
                }
            } else {
                div { margin_bottom: "var(--space-xl)" }
            }

            div {
                display: "flex",
                gap: "var(--space-md)",
                flex_wrap: "wrap",
                justify_content: "center",

                Link {
                    to: Route::Home {},
                    class: "btn btn-primary touch-target",
                    "Go Home"
                }

                Link {
                    to: Route::Explore {},
                    class: "btn btn-secondary touch-target",
                    "Explore Recipes"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn not_found_segments_empty_shows_slash() {
        let segments: Vec<String> = vec![];
        let path = if segments.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", segments.join("/"))
        };
        assert_eq!(path, "/");
    }

    #[test]
    fn not_found_segments_single() {
        let segments = vec!["foo".to_string()];
        let path = format!("/{}", segments.join("/"));
        assert_eq!(path, "/foo");
    }

    #[test]
    fn not_found_segments_nested() {
        let segments = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let path = format!("/{}", segments.join("/"));
        assert_eq!(path, "/a/b/c");
    }
}
