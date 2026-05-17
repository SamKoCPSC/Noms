use dioxus::prelude::*;

/// Size preset for the [`Avatar`] component.
#[derive(Default, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum AvatarSize {
    /// 32px — compact lists.
    Small,
    /// 48px — standard.
    #[default]
    Medium,
    /// 64px — profile headers.
    Large,
}

impl AvatarSize {
    fn pixel_size(self) -> u32 {
        match self {
            AvatarSize::Small => 32,
            AvatarSize::Medium => 48,
            AvatarSize::Large => 64,
        }
    }

    fn font_size(self) -> &'static str {
        match self {
            AvatarSize::Small => "12px",
            AvatarSize::Medium => "16px",
            AvatarSize::Large => "22px",
        }
    }
}

/// Props for the [`Avatar`] component.
#[derive(Props, Clone, PartialEq)]
pub struct AvatarProps {
    /// Image source. If `None`, falls back to initials.
    #[props(default)]
    pub src: Option<String>,
    /// Display size.
    #[props(default)]
    pub size: AvatarSize,
    /// Username used for initials fallback (e.g. "John Doe" → "JD").
    #[props(default)]
    pub username: String,
}

/// A circular avatar that shows an image or initials fallback.
#[component]
pub fn Avatar(props: AvatarProps) -> Element {
    let size = props.size.pixel_size();
    let font_size = props.size.font_size();
    let initials = extract_initials(&props.username);

    rsx! {
        div {
            width: "{size}px",
            height: "{size}px",
            border_radius: "50%",
            overflow: "hidden",
            background_color: "var(--accent)",
            color: "white",
            display: "flex",
            align_items: "center",
            justify_content: "center",
            font_size,
            font_weight: "600",
            font_family: "var(--font-display)",
            flex_shrink: "0",
            if let Some(src) = props.src {
                img {
                    src,
                    width: "100%",
                    height: "100%",
                    object_fit: "cover",
                    alt: "{props.username}",
                }
            } else {
                span { "{initials}" }
            }
        }
    }
}

/// Extract the first letter of the first two words from a username.
fn extract_initials(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return "?".to_string();
    }
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    let first = parts
        .first()
        .map(|s| s.chars().next().unwrap_or('?'))
        .unwrap_or('?');
    let second = parts
        .get(1)
        .map(|s| s.chars().next().unwrap_or('?'))
        .unwrap_or(first);
    format!("{}{}", first.to_uppercase(), second.to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::extract_initials;

    #[test]
    fn two_words() {
        assert_eq!(extract_initials("John Doe"), "JD");
    }

    #[test]
    fn single_word_doubles_first_letter() {
        assert_eq!(extract_initials("Sam"), "SS");
    }

    #[test]
    fn three_plus_words_uses_first_two() {
        assert_eq!(extract_initials("John Michael Doe"), "JM");
    }

    #[test]
    fn empty_string_returns_question_mark() {
        assert_eq!(extract_initials(""), "?");
    }

    #[test]
    fn whitespace_only_returns_question_mark() {
        assert_eq!(extract_initials("   "), "?");
    }

    #[test]
    fn trims_leading_and_trailing_whitespace() {
        assert_eq!(extract_initials("  John Doe  "), "JD");
    }

    #[test]
    fn handles_multiple_spaces_between_words() {
        assert_eq!(extract_initials("John   Doe"), "JD");
    }

    #[test]
    fn single_character_doubles_it() {
        assert_eq!(extract_initials("A"), "AA");
    }

    #[test]
    fn lowercased_input_is_uppercased() {
        assert_eq!(extract_initials("john doe"), "JD");
    }
}
