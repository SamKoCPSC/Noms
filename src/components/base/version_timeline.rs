//! Version timeline component.
//!
//! Displays a chronological list of recipe versions with visual indicators
//! for the current (latest) version.

use chrono::NaiveDateTime;
use dioxus::prelude::*;

use crate::components::base::{EmptyState, LoadingSpinner};

/// Summary of a single version for the timeline view.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct VersionSummary {
    pub version_number: i32,
    pub title: Option<String>,
    pub created_at: NaiveDateTime,
    pub is_latest: bool,
    pub notes: Option<String>,
}

/// Props for the VersionTimeline component.
#[derive(Props, Clone, PartialEq)]
pub struct VersionTimelineProps {
    pub versions: Vec<VersionSummary>,
    pub loading: bool,
    pub on_version_select: Callback<i32>,
    pub selected_version: Option<i32>,
    pub on_restore: Callback<i32>,
    pub restoring_version: Option<i32>,
}

/// Displays a vertical timeline of recipe versions.
///
/// Each item shows the version number, title, date, and whether it's the
/// current (latest) version. Clicking a version selects it for diff viewing.
#[component]
pub fn VersionTimeline(VersionTimelineProps { versions, loading, on_version_select, selected_version, on_restore, restoring_version }: VersionTimelineProps) -> Element {
    if loading {
        return rsx! {
            div {
                class: "flex items-center justify-center py-8",
                LoadingSpinner {}
                p {
                    class: "text-sm text-muted-foreground mt-2",
                    "Loading version history..."
                }
            }
        };
    }

    if versions.is_empty() {
        return rsx! {
            EmptyState {
                icon: rsx! { span { "📋" } },
                title: "No version history",
                description: "This recipe has only one version. Edit the recipe to create a new version.",
            }
        };
    }

    rsx! {
        div {
            class: "version-timeline space-y-0",
            for version in versions.iter().cloned() {
                TimelineItem {
                    version: version.clone(),
                    selected: selected_version == Some(version.version_number),
                    on_select: Callback::new(move |_| {
                        on_version_select.call(version.version_number);
                    }),
                    on_restore: if !version.is_latest {
                        Some(Callback::new(move |_| {
                            on_restore.call(version.version_number);
                        }))
                    } else {
                        None
                    },
                    is_restoring: restoring_version == Some(version.version_number),
                }
            }
        }
    }
}

/// Props for a single timeline item.
#[derive(Props, Clone, PartialEq)]
struct TimelineItemProps {
    version: VersionSummary,
    selected: bool,
    on_select: Callback<()>,
    on_restore: Option<Callback<()>>,
    is_restoring: bool,
}

/// A single version entry in the timeline.
#[component]
fn TimelineItem(TimelineItemProps { version, selected, on_select, on_restore, is_restoring }: TimelineItemProps) -> Element {
    let display_title = version.title.as_deref().unwrap_or("Untitled");
    let date_str = version.created_at.format("%b %d, %Y at %I:%M %p").to_string();

    let item_class = if selected {
        "timeline-item flex items-start gap-3 p-3 rounded-lg transition-colors bg-accent"
    } else {
        "timeline-item flex items-start gap-3 p-3 rounded-lg transition-colors hover:bg-muted/50"
    };

    let dot_class = if version.is_latest {
        "w-3 h-3 rounded-full flex-shrink-0 bg-primary"
    } else {
        "w-3 h-3 rounded-full flex-shrink-0 bg-border"
    };

    rsx! {
        div {
            class: item_class,
            // Timeline dot and line
            div {
                class: "flex flex-col items-center",
                div {
                    class: dot_class,
                }
                div {
                    class: "w-px flex-1 bg-border min-h-[20px]",
                }
            }
            // Content
            div {
                class: "flex-1 min-w-0",
                div {
                    class: "flex items-center gap-2",
                    span {
                        class: "text-xs font-mono text-muted-foreground",
                        "v{version.version_number}"
                    }
                    if version.is_latest {
                        span {
                            class: "text-xs px-1.5 py-0.5 rounded-full bg-primary/10 text-primary font-medium",
                            "Current"
                        }
                    }
                }
                p {
                    class: "text-sm font-medium truncate",
                    "{display_title}"
                }
                p {
                    class: "text-xs text-muted-foreground",
                    "{date_str}"
                }
                if let Some(ref notes) = version.notes {
                    if !notes.is_empty() {
                        p {
                            class: "text-xs text-muted-foreground italic mt-0.5 truncate",
                            "{notes}"
                        }
                    }
                }
                button {
                    class: "btn btn-ghost touch-target text-xs mt-1",
                    disabled: selected,
                    r#"type"#: "button",
                    onclick: move |_| on_select.call(()),
                    if selected {
                        "Viewing"
                    } else {
                        "View"
                    }
                }
                if let Some(restore_cb) = on_restore {
                    button {
                        class: "restore-btn touch-target text-xs mt-1",
                        r#"type"#: "button",
                        disabled: is_restoring,
                        onclick: move |_| restore_cb.call(()),
                        if is_restoring {
                            "Restoring..."
                        } else {
                            "Restore"
                        }
                    }
                }
            }
        }
    }
}
