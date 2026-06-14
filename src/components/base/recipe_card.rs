//! Recipe card for the dashboard grid.
//!
//! Displays a compact preview of a recipe: title, description snippet,
//! and relative creation date. Clicking navigates to the recipe detail page.

use dioxus::prelude::*;

use crate::types::Recipe;

/// Props for the [`RecipeCard`] component.
#[derive(Props, Clone, PartialEq)]
pub struct RecipeCardProps {
    /// The recipe to display.
    pub recipe: Recipe,
}

/// A clickable card showing a recipe preview.
///
/// Clicking the card navigates to the recipe detail page.
#[component]
pub fn RecipeCard(props: RecipeCardProps) -> Element {
    let RecipeCardProps { recipe } = props;
    let id = recipe.id.to_string();

    // Truncate description to ~120 characters
    let description_snippet = recipe
        .description
        .as_deref()
        .unwrap_or("")
        .chars()
        .take(120)
        .collect::<String>();

    let relative_time = format_relative_time(&recipe.created_at);

    rsx! {
        Link {
            to: crate::Route::RecipeDetail { id },
            class: "recipe-card",
            div {
                class: "recipe-card__content",
                h3 {
                    class: "recipe-card__title",
                    "{recipe.title}"
                }
                if !description_snippet.is_empty() {
                    p {
                        class: "recipe-card__description",
                        "{description_snippet}"
                    }
                }
                span {
                    class: "recipe-card__meta",
                    "Created {relative_time}"
                }
            }
        }
    }
}

/// Format a UTC datetime as a relative time string.
fn format_relative_time(dt: &chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let diff = now.signed_duration_since(*dt);

    if diff.num_days() > 0 {
        let days = diff.num_days();
        format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
    } else if diff.num_hours() > 0 {
        let hours = diff.num_hours();
        format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
    } else if diff.num_minutes() > 0 {
        let minutes = diff.num_minutes();
        format!(
            "{} minute{} ago",
            minutes,
            if minutes == 1 { "" } else { "s" }
        )
    } else {
        "Just now".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_format_relative_time_just_now() {
        let now = chrono::Utc::now();
        assert_eq!(format_relative_time(&now), "Just now");
    }

    #[test]
    fn test_format_relative_time_minutes() {
        let dt = chrono::Utc::now() - Duration::minutes(5);
        assert_eq!(format_relative_time(&dt), "5 minutes ago");
    }

    #[test]
    fn test_format_relative_time_one_minute() {
        let dt = chrono::Utc::now() - Duration::minutes(1);
        assert_eq!(format_relative_time(&dt), "1 minute ago");
    }

    #[test]
    fn test_format_relative_time_hours() {
        let dt = chrono::Utc::now() - Duration::hours(3);
        assert_eq!(format_relative_time(&dt), "3 hours ago");
    }

    #[test]
    fn test_format_relative_time_one_hour() {
        let dt = chrono::Utc::now() - Duration::hours(1);
        assert_eq!(format_relative_time(&dt), "1 hour ago");
    }

    #[test]
    fn test_format_relative_time_days() {
        let dt = chrono::Utc::now() - Duration::days(7);
        assert_eq!(format_relative_time(&dt), "7 days ago");
    }

    #[test]
    fn test_format_relative_time_one_day() {
        let dt = chrono::Utc::now() - Duration::days(1);
        assert_eq!(format_relative_time(&dt), "1 day ago");
    }
}
