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

/// Geometry used to clip an edited image after applying its crop rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageCropShape {
    #[default]
    Rectangle,
    Ellipse,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeImageEdit {
    pub name: String,
    pub version: u32,
    #[serde(default)]
    pub crop_shape: ImageCropShape,
    pub aspect_ratio: ImageAspectRatio,
    pub crop: ImageCropRect,
    pub transform: ImageTransformEdit,
    pub color: ImageColorEdit,
}

impl RecipeImageEdit {
    pub fn default_rect_16x9() -> Self {
        Self::template("rectangle:16:9", ImageCropShape::Rectangle, 16, 9)
    }

    pub fn default_ellipse_1x1() -> Self {
        Self::template("ellipse:1:1", ImageCropShape::Ellipse, 1, 1)
    }

    fn template(
        name: &str,
        crop_shape: ImageCropShape,
        aspect_width: u32,
        aspect_height: u32,
    ) -> Self {
        Self {
            name: name.to_string(),
            version: 1,
            crop_shape,
            aspect_ratio: ImageAspectRatio {
                width: aspect_width,
                height: aspect_height,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<u64>,
    pub edits: Vec<RecipeImageEdit>,
}

impl RecipeImage {
    pub fn new(url: String, thumb_url: Option<String>, edit: RecipeImageEdit) -> Self {
        Self {
            url,
            thumb_url,
            file_name: None,
            file_size: None,
            edits: vec![edit],
        }
    }

    pub fn with_file_metadata(mut self, file_name: String, file_size: u64) -> Self {
        self.file_name = Some(file_name);
        self.file_size = Some(file_size);
        self
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
                file_name: Option<String>,
                #[serde(default)]
                file_size: Option<u64>,
                #[serde(default)]
                edits: Vec<RecipeImageEdit>,
            },
        }

        match RecipeImageCompat::deserialize(deserializer)? {
            RecipeImageCompat::Url(url) => Ok(Self {
                url,
                thumb_url: None,
                file_name: None,
                file_size: None,
                edits: Vec::new(),
            }),
            RecipeImageCompat::Object {
                url,
                thumb_url,
                file_name,
                file_size,
                edits,
            } => Ok(Self {
                url,
                thumb_url,
                file_name,
                file_size,
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
    #[serde(default)]
    pub sub_steps: Vec<RecipeStep>,
    /// Structured step photos with full/thumb URLs and edit metadata.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub photos: Vec<RecipeImage>,
}

impl RecipeStep {
    pub fn display_photos(&self) -> Vec<RecipeImage> {
        self.photos.clone()
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

/// Client-facing recipe editor state. The server converts this into its
/// canonical persistence snapshot; IDs and presentation metadata stay out of it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeEditorState {
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
    pub tags: Vec<String>,
}

impl Default for RecipeEditorState {
    fn default() -> Self {
        Self {
            title: String::new(),
            description: None,
            commentary: None,
            prep_time_minutes: None,
            cook_time_minutes: None,
            servings: None,
            ingredients: Vec::new(),
            instructions: Vec::new(),
            equipment: Vec::new(),
            images: Vec::new(),
            visibility: "private".to_string(),
            tags: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeDraftView {
    pub id: Uuid,
    pub recipe_id: Option<Uuid>,
    pub base_version_number: Option<i32>,
    pub revision: i64,
    pub editor: RecipeEditorState,
    pub forked_from_recipe_id: Option<Uuid>,
    pub forked_from_version_number: Option<i32>,
    pub forked_from_title: Option<String>,
    pub fork_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeVersionSummary {
    pub version_number: i32,
    pub title: String,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub is_current: bool,
    pub change_summary: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeVersionView {
    pub version_number: i32,
    pub current_version_number: i32,
    pub editor: RecipeEditorState,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub changes_from_previous: Vec<RecipeVersionChange>,
    pub changes_to_current: Vec<RecipeVersionChange>,
    pub changes_from_original: Vec<RecipeVersionChange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeVersionChange {
    pub op: String,
    pub path: String,
    /// Display-ready values avoid decoding arbitrary JSON trees in the
    /// browser while rendering a comparison.
    pub old_value: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeVersionComparison {
    pub from_version_number: i32,
    pub to_version_number: i32,
    pub changes: Vec<RecipeVersionChange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForkAttribution {
    pub original_recipe_id: Option<Uuid>,
    pub original_title: String,
    pub message: Option<String>,
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
    fn rectangular_and_elliptical_edit_templates_are_explicit() {
        let rectangle = RecipeImageEdit::default_rect_16x9();
        let ellipse = RecipeImageEdit::default_ellipse_1x1();

        assert_eq!(rectangle.name, "rectangle:16:9");
        assert_eq!(rectangle.crop_shape, ImageCropShape::Rectangle);
        assert_eq!(
            rectangle.aspect_ratio,
            ImageAspectRatio {
                width: 16,
                height: 9
            }
        );

        assert_eq!(ellipse.name, "ellipse:1:1");
        assert_eq!(ellipse.crop_shape, ImageCropShape::Ellipse);
        assert_eq!(
            ellipse.aspect_ratio,
            ImageAspectRatio {
                width: 1,
                height: 1
            }
        );
    }

    #[test]
    fn legacy_edit_without_crop_shape_defaults_to_rectangle() {
        let edit = serde_json::from_value::<RecipeImageEdit>(serde_json::json!({
            "name": "16:9",
            "version": 1,
            "aspect_ratio": { "width": 16, "height": 9 },
            "crop": { "x": 0.0, "y": 0.0, "width": 1.0, "height": 1.0 },
            "transform": {
                "rotation": 0.0,
                "stretch_x": 1.0,
                "stretch_y": 1.0,
                "flip_x": false,
                "flip_y": false
            },
            "color": {
                "saturation": 1.0,
                "contrast_r": 1.0,
                "contrast_g": 1.0,
                "contrast_b": 1.0
            }
        }))
        .unwrap();

        assert_eq!(edit.crop_shape, ImageCropShape::Rectangle);
    }

    #[test]
    fn recipe_image_deserializes_legacy_edit_without_crop_shape() {
        let image = serde_json::from_value::<RecipeImage>(serde_json::json!({
            "url": "https://example.com/legacy.webp",
            "edits": [{
                "name": "16:9",
                "version": 1,
                "aspect_ratio": { "width": 16, "height": 9 },
                "crop": { "x": 0.0, "y": 0.0, "width": 1.0, "height": 1.0 },
                "transform": {
                    "rotation": 0.0,
                    "stretch_x": 1.0,
                    "stretch_y": 1.0,
                    "flip_x": false,
                    "flip_y": false
                },
                "color": {
                    "saturation": 1.0,
                    "contrast_r": 1.0,
                    "contrast_g": 1.0,
                    "contrast_b": 1.0
                }
            }]
        }))
        .unwrap();

        assert_eq!(image.edits[0].crop_shape, ImageCropShape::Rectangle);
    }

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
            RecipeImageEdit::default_rect_16x9(),
        );

        let value = serde_json::to_value(image).unwrap();

        assert_eq!(value["url"], "https://example.com/image_full.webp");
        assert_eq!(value["thumb_url"], "https://example.com/image_thumb.webp");
        assert_eq!(value["edits"][0]["name"], "rectangle:16:9");
        assert_eq!(value["edits"][0]["version"], 1);
        assert_eq!(value["edits"][0]["aspect_ratio"]["width"], 16);
        assert_eq!(value["edits"][0]["aspect_ratio"]["height"], 9);
        assert_eq!(value["edits"][0]["crop"]["x"], 0.0);
        assert_eq!(value["edits"][0]["transform"]["rotation"], 0.0);
        assert_eq!(value["edits"][0]["color"]["saturation"], 1.0);
    }

    #[test]
    fn recipe_image_file_metadata_round_trips() {
        let image = RecipeImage::new(
            "https://example.com/image_full.webp".to_string(),
            None,
            RecipeImageEdit::default_rect_16x9(),
        )
        .with_file_metadata("family-recipe.jpg".to_string(), 2_400_000);

        let serialized = serde_json::to_value(&image).unwrap();
        let restored: RecipeImage = serde_json::from_value(serialized).unwrap();

        assert_eq!(restored.file_name.as_deref(), Some("family-recipe.jpg"));
        assert_eq!(restored.file_size, Some(2_400_000));
    }

    #[test]
    fn recipe_image_display_url_prefers_thumbnail_when_available() {
        let image = RecipeImage::new(
            "https://example.com/image_full.webp".to_string(),
            Some("https://example.com/image_thumb.webp".to_string()),
            RecipeImageEdit::default_rect_16x9(),
        );

        assert_eq!(
            image.display_url(false),
            "https://example.com/image_full.webp"
        );
        assert_eq!(
            image.display_url(true),
            "https://example.com/image_thumb.webp"
        );
    }

    #[test]
    fn recipe_step_display_photos_reads_multiple_photos() {
        let step = RecipeStep {
            text: "Mix".to_string(),
            sub_steps: Vec::new(),
            photos: vec![
                RecipeImage {
                    url: "https://example.com/step_1.webp".to_string(),
                    thumb_url: None,
                    file_name: None,
                    file_size: None,
                    edits: Vec::new(),
                },
                RecipeImage {
                    url: "https://example.com/step_2.webp".to_string(),
                    thumb_url: None,
                    file_name: None,
                    file_size: None,
                    edits: Vec::new(),
                },
            ],
        };

        let photos = step.display_photos();

        assert_eq!(photos.len(), 2);
        assert_eq!(photos[0].url, "https://example.com/step_1.webp");
        assert_eq!(photos[1].url, "https://example.com/step_2.webp");
    }
}
