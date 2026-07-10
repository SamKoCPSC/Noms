//! Shared types used across the client-server boundary.
//!
//! These types are serialized/deserialized by Dioxus `#[server]` functions
//! and must be available on both the client (wasm) and server.
#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageAspectRatio {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageCropRect {
    /// Left edge in normalized original-image coordinates, 0.0 to 1.0.
    pub x: f64,
    /// Top edge in normalized original-image coordinates, 0.0 to 1.0.
    pub y: f64,
    /// Width in normalized original-image coordinates, 0.0 to 1.0.
    pub width: f64,
    /// Height in normalized original-image coordinates, 0.0 to 1.0.
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageTransformEdit {
    pub rotation: f64,
    pub stretch_x: f64,
    pub stretch_y: f64,
    pub flip_x: bool,
    pub flip_y: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageColorEdit {
    pub saturation: f64,
    pub contrast_r: f64,
    pub contrast_g: f64,
    pub contrast_b: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeImageEdit {
    pub name: String,
    pub version: u32,
    pub aspect_ratio: ImageAspectRatio,
    pub crop: ImageCropRect,
    pub transform: ImageTransformEdit,
    pub color: ImageColorEdit,
}

impl RecipeImageEdit {
    pub fn default_16x9() -> Self {
        Self {
            name: "16:9".to_string(),
            version: 1,
            aspect_ratio: ImageAspectRatio {
                width: 16,
                height: 9,
            },
            crop: ImageCropRect {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
            transform: ImageTransformEdit {
                rotation: 0.0,
                stretch_x: 1.0,
                stretch_y: 1.0,
                flip_x: false,
                flip_y: false,
            },
            color: ImageColorEdit {
                saturation: 1.0,
                contrast_r: 1.0,
                contrast_g: 1.0,
                contrast_b: 1.0,
            },
        }
    }
}

/// A recipe image stored in `recipes.images`.
///
/// Older rows stored image entries as plain URL strings. The custom
/// deserializer keeps those rows readable by converting each string to a
/// structured image record with no saved edits.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecipeImage {
    /// Full-size image URL.
    pub url: String,
    /// Smaller image variant for compact UI surfaces.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb_url: Option<String>,
    pub edits: Vec<RecipeImageEdit>,
}

impl RecipeImage {
    pub fn new(url: String, thumb_url: Option<String>, edit: RecipeImageEdit) -> Self {
        Self {
            url,
            thumb_url,
            edits: vec![edit],
        }
    }

    pub fn display_url(&self, prefer_thumbnail: bool) -> &str {
        if prefer_thumbnail {
            self.thumb_url.as_deref().unwrap_or(&self.url)
        } else {
            &self.url
        }
    }
}

impl<'de> Deserialize<'de> for RecipeImage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum RecipeImageCompat {
            Url(String),
            Object {
                url: String,
                #[serde(default)]
                thumb_url: Option<String>,
                #[serde(default)]
                edits: Vec<RecipeImageEdit>,
            },
        }

        match RecipeImageCompat::deserialize(deserializer)? {
            RecipeImageCompat::Url(url) => Ok(Self {
                url,
                thumb_url: None,
                edits: Vec::new(),
            }),
            RecipeImageCompat::Object {
                url,
                thumb_url,
                edits,
            } => Ok(Self {
                url,
                thumb_url,
                edits,
            }),
        }
    }
}

/// A single ingredient in a recipe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeIngredient {
    pub amount: String,
    pub unit: String,
    pub name: String,
}

/// A single step in a recipe (supports nested sub-steps).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeStep {
    pub text: String,
    pub sub_steps: Vec<RecipeStep>,
    /// Optional photo URL for this step (full-size image URL from storage).
    pub photo_url: Option<String>,
    /// Structured step photo with full/thumb URLs and edit metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub photo: Option<RecipeImage>,
}

impl RecipeStep {
    pub fn display_photo(&self) -> Option<RecipeImage> {
        self.photo.clone().or_else(|| {
            self.photo_url.as_ref().map(|url| RecipeImage {
                url: url.clone(),
                thumb_url: None,
                edits: Vec::new(),
            })
        })
    }
}

/// A piece of equipment needed for a recipe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeEquipment {
    pub name: String,
}

/// A recipe saved by a user.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub commentary: Option<String>,
    pub prep_time_minutes: Option<i32>,
    pub cook_time_minutes: Option<i32>,
    pub servings: Option<i32>,
    pub ingredients: Vec<RecipeIngredient>,
    pub instructions: Vec<RecipeStep>,
    pub equipment: Vec<RecipeEquipment>,
    pub images: Vec<RecipeImage>,
    pub visibility: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub author_username: String,
    pub author_avatar_url: Option<String>,
}

/// Paginated recipe list response for server functions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeListResponse {
    pub recipes: Vec<Recipe>,
    pub total_count: i64,
    pub has_more: bool,
}

/// Public user profile with recipe count.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub public_recipe_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipe_image_deserializes_legacy_url_string() {
        let image: RecipeImage =
            serde_json::from_value(serde_json::json!("https://example.com/image.webp")).unwrap();

        assert_eq!(image.url, "https://example.com/image.webp");
        assert_eq!(image.thumb_url, None);
        assert!(image.edits.is_empty());
    }

    #[test]
    fn recipe_image_deserializes_object_without_thumbnail() {
        let image: RecipeImage = serde_json::from_value(serde_json::json!({
            "url": "https://example.com/image_full.webp",
            "edits": []
        }))
        .unwrap();

        assert_eq!(image.url, "https://example.com/image_full.webp");
        assert_eq!(image.thumb_url, None);
        assert!(image.edits.is_empty());
    }

    #[test]
    fn recipe_image_serializes_urls_with_named_edit() {
        let image = RecipeImage::new(
            "https://example.com/image_full.webp".to_string(),
            Some("https://example.com/image_thumb.webp".to_string()),
            RecipeImageEdit::default_16x9(),
        );

        let value = serde_json::to_value(image).unwrap();

        assert_eq!(value["url"], "https://example.com/image_full.webp");
        assert_eq!(value["thumb_url"], "https://example.com/image_thumb.webp");
        assert_eq!(value["edits"][0]["name"], "16:9");
        assert_eq!(value["edits"][0]["version"], 1);
        assert_eq!(value["edits"][0]["aspect_ratio"]["width"], 16);
        assert_eq!(value["edits"][0]["aspect_ratio"]["height"], 9);
        assert_eq!(value["edits"][0]["crop"]["x"], 0.0);
        assert_eq!(value["edits"][0]["transform"]["rotation"], 0.0);
        assert_eq!(value["edits"][0]["color"]["saturation"], 1.0);
    }

    #[test]
    fn recipe_image_display_url_prefers_thumbnail_when_available() {
        let image = RecipeImage::new(
            "https://example.com/image_full.webp".to_string(),
            Some("https://example.com/image_thumb.webp".to_string()),
            RecipeImageEdit::default_16x9(),
        );

        assert_eq!(image.display_url(false), "https://example.com/image_full.webp");
        assert_eq!(
            image.display_url(true),
            "https://example.com/image_thumb.webp"
        );
    }
}
