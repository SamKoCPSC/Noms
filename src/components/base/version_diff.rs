//! Version diff component.
//!
//! Displays a reconstructed version of a recipe, with field-level comparison
//! against the current (latest) version.

use chrono::NaiveDateTime;
use dioxus::prelude::*;

use crate::components::base::{EmptyState, LoadingSpinner};

/// A reconstructed version of a recipe for display.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct ReconstructedVersion {
    pub version_number: i32,
    pub title: String,
    pub description: Option<String>,
    pub prep_time_min: Option<i32>,
    pub cook_time_min: Option<i32>,
    pub total_time_min: Option<i32>,
    pub servings: Option<i32>,
    pub ingredients: Option<serde_json::Value>,
    pub steps: Option<serde_json::Value>,
    pub created_at: NaiveDateTime,
    pub notes: Option<String>,
}

/// Props for the VersionDiff component.
#[derive(Props, Clone, PartialEq)]
pub struct VersionDiffProps {
    pub version: Option<ReconstructedVersion>,
    pub loading: bool,
    pub error: Option<String>,
}

/// Displays a reconstructed version with all recipe fields.
///
/// Shows a loading spinner while fetching, an error message if reconstruction
/// fails, or the full recipe data for the selected version.
#[component]
pub fn VersionDiff(VersionDiffProps { version, loading, error }: VersionDiffProps) -> Element {
    if loading {
        return rsx! {
            div {
                class: "flex items-center justify-center py-12",
                LoadingSpinner {}
                p {
                    class: "text-sm text-muted-foreground mt-4",
                    "Reconstructing version..."
                }
            }
        };
    }

    if let Some(ref err) = error {
        return rsx! {
            EmptyState {
                icon: rsx! { span { "⚠️" } },
                title: "Failed to load version",
                description: err.clone(),
            }
        };
    }

    let Some(ver) = version else {
        return rsx! {
            EmptyState {
                icon: rsx! { span { "🔍" } },
                title: "Select a version",
                description: "Click a version from the timeline to view its contents.",
            }
        };
    };

    let date_str = ver.created_at.format("%b %d, %Y at %I:%M %p").to_string();
    let version_header = format!("Version {}", ver.version_number);

    rsx! {
        div {
            class: "version-diff space-y-6",
            // Header
            div {
                class: "flex items-center justify-between",
                div {
                    h3 {
                        class: "text-lg font-semibold",
                        "{version_header}"
                    }
                    p {
                        class: "text-sm text-muted-foreground",
                        "{date_str}"
                    }
                }
            }

            // Notes
            if let Some(ref notes) = ver.notes {
                if !notes.is_empty() {
                    div {
                        class: "neumo-card border-amber-200 bg-amber-50/50",
                        padding: "var(--space-lg)",
                        background_color: "var(--surface)",
                        div {
                            class: "text-sm text-amber-800",
                            p {
                                class: "font-medium mb-1",
                                "Version Notes"
                            }
                            p { "{notes}" }
                        }
                    }
                }
            }

            // Title
            div {
                class: "neumo-card diff-section",
                padding: "var(--space-lg)",
                background_color: "var(--surface)",
                div {
                    class: "space-y-1",
                    p {
                        class: "text-xs font-medium text-muted-foreground uppercase tracking-wider",
                        "Title"
                    }
                    p {
                        class: "text-base",
                        "{ver.title}"
                    }
                }
            }

            // Description
            if let Some(ref desc) = ver.description {
                if !desc.is_empty() {
                    div {
                        class: "neumo-card diff-section",
                        padding: "var(--space-lg)",
                        background_color: "var(--surface)",
                        div {
                            class: "space-y-1",
                            p {
                                class: "text-xs font-medium text-muted-foreground uppercase tracking-wider",
                                "Description"
                            }
                            p {
                                class: "text-sm",
                                "{desc}"
                            }
                        }
                    }
                }
            }

            // Time info
            div {
                class: "grid grid-cols-2 sm:grid-cols-4 gap-4",
                if let Some(prep) = ver.prep_time_min {
                    div {
                        class: "neumo-card diff-section",
                        padding: "var(--space-lg)",
                        background_color: "var(--surface)",
                        div {
                            class: "space-y-1",
                            p {
                                class: "text-xs font-medium text-muted-foreground uppercase tracking-wider",
                                "Prep"
                            }
                            p {
                                class: "text-base font-medium",
                                "{prep} min"
                            }
                        }
                    }
                }
                if let Some(cook) = ver.cook_time_min {
                    div {
                        class: "neumo-card diff-section",
                        padding: "var(--space-lg)",
                        background_color: "var(--surface)",
                        div {
                            class: "space-y-1",
                            p {
                                class: "text-xs font-medium text-muted-foreground uppercase tracking-wider",
                                "Cook"
                            }
                            p {
                                class: "text-base font-medium",
                                "{cook} min"
                            }
                        }
                    }
                }
                if let Some(total) = ver.total_time_min {
                    div {
                        class: "neumo-card diff-section",
                        padding: "var(--space-lg)",
                        background_color: "var(--surface)",
                        div {
                            class: "space-y-1",
                            p {
                                class: "text-xs font-medium text-muted-foreground uppercase tracking-wider",
                                "Total"
                            }
                            p {
                                class: "text-base font-medium",
                                "{total} min"
                            }
                        }
                    }
                }
                if let Some(servings) = ver.servings {
                    div {
                        class: "neumo-card diff-section",
                        padding: "var(--space-lg)",
                        background_color: "var(--surface)",
                        div {
                            class: "space-y-1",
                            p {
                                class: "text-xs font-medium text-muted-foreground uppercase tracking-wider",
                                "Servings"
                            }
                            p {
                                class: "text-base font-medium",
                                "{servings}"
                            }
                        }
                    }
                }
            }

            // Ingredients
            if let Some(ref ingredients) = ver.ingredients {
                if let Some(arr) = ingredients.as_array() {
                    if !arr.is_empty() {
                        div {
                            class: "neumo-card diff-section",
                            padding: "var(--space-lg)",
                            background_color: "var(--surface)",
                            div {
                                class: "space-y-2",
                                p {
                                    class: "text-xs font-medium text-muted-foreground uppercase tracking-wider",
                                    "Ingredients"
                                }
                                ul {
                                    class: "list-disc list-inside space-y-1",
                                    for ingredient in arr.iter() {
                                        li {
                                            class: "text-sm",
                                            "{ingredient_to_string(ingredient)}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Steps
            if let Some(ref steps) = ver.steps {
                if let Some(arr) = steps.as_array() {
                    if !arr.is_empty() {
                        div {
                            class: "neumo-card diff-section",
                            padding: "var(--space-lg)",
                            background_color: "var(--surface)",
                            div {
                                class: "space-y-2",
                                p {
                                    class: "text-xs font-medium text-muted-foreground uppercase tracking-wider",
                                    "Steps"
                                }
                                ol {
                                    class: "list-decimal list-inside space-y-2",
                                    for step in arr.iter() {
                                        li {
                                            class: "text-sm",
                                            "{step_to_string(step)}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Convert an ingredient JSON value to a display string.
fn ingredient_to_string(val: &serde_json::Value) -> String {
    if let s @ serde_json::Value::String(_) = val {
        s.as_str().unwrap_or("").to_string()
    } else if let Some(obj) = val.as_object() {
        if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
            if let Some(amount) = obj.get("amount").and_then(|v| v.as_str()) {
                format!("{} - {}", amount, name)
            } else {
                name.to_string()
            }
        } else {
            serde_json::to_string(val).unwrap_or_default()
        }
    } else {
        serde_json::to_string(val).unwrap_or_default()
    }
}

/// Convert a step JSON value to a display string.
fn step_to_string(val: &serde_json::Value) -> String {
    if let s @ serde_json::Value::String(_) = val {
        s.as_str().unwrap_or("").to_string()
    } else if let Some(obj) = val.as_object() {
        if let Some(instruction) = obj.get("instruction").and_then(|v| v.as_str()) {
            instruction.to_string()
        } else {
            serde_json::to_string(val).unwrap_or_default()
        }
    } else {
        serde_json::to_string(val).unwrap_or_default()
    }
}
