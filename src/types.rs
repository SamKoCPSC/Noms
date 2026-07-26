//! Shared types used across the client-server boundary.
//!
//! These types are serialized/deserialized by Dioxus `#[server]` functions
//! and must be available on both the client (wasm) and server.
#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
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
    /// First image captured in this immutable snapshot.
    pub image_url: Option<String>,
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
    pub comparisons_available: bool,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeLineageNode {
    /// Present only when the viewer may navigate to the live recipe.
    pub recipe_id: Option<Uuid>,
    pub title: String,
    pub owner_username: Option<String>,
    /// First recipe thumbnail, present only when the live recipe is accessible.
    pub image_url: Option<String>,
    pub forked_from_version_number: Option<i32>,
    pub fork_message: Option<String>,
    pub is_deleted: bool,
    pub is_redacted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeLineageBranch {
    pub node: RecipeLineageNode,
    pub children: Vec<RecipeLineageBranch>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeLineageView {
    /// Ordered from the root ancestor to the current recipe's direct parent.
    pub ancestors: Vec<RecipeLineageNode>,
    pub current: RecipeLineageNode,
    pub descendants: Vec<RecipeLineageBranch>,
    pub truncated: bool,
    pub node_limit_reached: bool,
}

/// Stable identity for one immutable recipe snapshot in a version graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VersionGraphNodeId {
    pub recipe_id: Uuid,
    pub version_number: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionGraphEdgeKind {
    Version,
    Fork,
}

/// Directed relationship between two normalized snapshot nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VersionGraphEdge {
    source: VersionGraphNodeId,
    target: VersionGraphNodeId,
    kind: VersionGraphEdgeKind,
}

impl VersionGraphEdge {
    /// Connect two consecutive snapshots belonging to the same recipe.
    pub fn version(source: VersionGraphNodeId, target: VersionGraphNodeId) -> Option<Self> {
        let expected_target_version = source.version_number.checked_add(1)?;
        (source.version_number > 0
            && source.recipe_id == target.recipe_id
            && target.version_number == expected_target_version)
            .then_some(Self {
                source,
                target,
                kind: VersionGraphEdgeKind::Version,
            })
    }

    /// Connect the recorded source snapshot to the first snapshot of a fork.
    pub fn fork(
        source_recipe_id: Uuid,
        forked_version_number: i32,
        forked_recipe_id: Uuid,
    ) -> Option<Self> {
        (forked_version_number > 0 && source_recipe_id != forked_recipe_id).then_some(Self {
            source: VersionGraphNodeId {
                recipe_id: source_recipe_id,
                version_number: forked_version_number,
            },
            target: VersionGraphNodeId {
                recipe_id: forked_recipe_id,
                version_number: 1,
            },
            kind: VersionGraphEdgeKind::Fork,
        })
    }

    pub fn source(&self) -> VersionGraphNodeId {
        self.source
    }

    pub fn target(&self) -> VersionGraphNodeId {
        self.target
    }

    pub fn kind(&self) -> VersionGraphEdgeKind {
        self.kind
    }
}

#[derive(Deserialize)]
struct VersionGraphEdgeWire {
    source: VersionGraphNodeId,
    target: VersionGraphNodeId,
    kind: VersionGraphEdgeKind,
}

impl<'de> Deserialize<'de> for VersionGraphEdge {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = VersionGraphEdgeWire::deserialize(deserializer)?;
        let edge = match wire.kind {
            VersionGraphEdgeKind::Version => Self::version(wire.source, wire.target),
            VersionGraphEdgeKind::Fork if wire.target.version_number == 1 => Self::fork(
                wire.source.recipe_id,
                wire.source.version_number,
                wire.target.recipe_id,
            ),
            VersionGraphEdgeKind::Fork => None,
        };

        edge.filter(|edge| edge.source == wire.source && edge.target == wire.target)
            .ok_or_else(|| D::Error::custom("invalid version graph edge"))
    }
}

/// Non-sensitive state retained even when graph node content is redacted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionGraphNodeState {
    pub is_selected_recipe: bool,
    pub is_current_version: bool,
    pub is_deleted: bool,
}

/// Snapshot metadata available to the current viewer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionGraphNodeContent {
    pub title: String,
    pub owner_username: Option<String>,
    /// Snapshot image and its non-destructive edit metadata.
    pub image: Option<RecipeImage>,
    pub created_at: DateTime<Utc>,
    pub change_summary: String,
}

/// Display metadata for one immutable recipe snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct VersionGraphNode {
    id: VersionGraphNodeId,
    title: Option<String>,
    owner_username: Option<String>,
    image: Option<RecipeImage>,
    created_at: Option<DateTime<Utc>>,
    change_summary: Option<String>,
    is_selected_recipe: bool,
    is_current_version: bool,
    is_deleted: bool,
    is_redacted: bool,
}

impl VersionGraphNode {
    pub fn visible(
        id: VersionGraphNodeId,
        content: VersionGraphNodeContent,
        state: VersionGraphNodeState,
    ) -> Self {
        Self {
            id,
            title: Some(content.title),
            owner_username: content.owner_username,
            image: content.image,
            created_at: Some(content.created_at),
            change_summary: Some(content.change_summary),
            is_selected_recipe: state.is_selected_recipe,
            is_current_version: state.is_current_version,
            is_deleted: state.is_deleted,
            is_redacted: false,
        }
    }

    /// Preserve graph topology and non-sensitive state without exposing content.
    pub fn redacted(id: VersionGraphNodeId, state: VersionGraphNodeState) -> Self {
        Self {
            id,
            title: None,
            owner_username: None,
            image: None,
            created_at: None,
            change_summary: None,
            is_selected_recipe: state.is_selected_recipe,
            is_current_version: state.is_current_version,
            is_deleted: state.is_deleted,
            is_redacted: true,
        }
    }

    pub fn id(&self) -> VersionGraphNodeId {
        self.id
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn owner_username(&self) -> Option<&str> {
        self.owner_username.as_deref()
    }

    pub fn image(&self) -> Option<&RecipeImage> {
        self.image.as_ref()
    }

    pub fn thumbnail_url(&self) -> Option<&str> {
        self.image.as_ref().map(|image| image.display_url(true))
    }

    pub fn created_at(&self) -> Option<DateTime<Utc>> {
        self.created_at
    }

    pub fn change_summary(&self) -> Option<&str> {
        self.change_summary.as_deref()
    }

    pub fn is_selected_recipe(&self) -> bool {
        self.is_selected_recipe
    }

    pub fn is_current_version(&self) -> bool {
        self.is_current_version
    }

    pub fn is_deleted(&self) -> bool {
        self.is_deleted
    }

    pub fn is_redacted(&self) -> bool {
        self.is_redacted
    }
}

#[derive(Serialize, Deserialize)]
struct VersionGraphNodeWire {
    id: VersionGraphNodeId,
    title: Option<String>,
    owner_username: Option<String>,
    image: Option<RecipeImage>,
    created_at: Option<DateTime<Utc>>,
    change_summary: Option<String>,
    is_selected_recipe: bool,
    is_current_version: bool,
    is_deleted: bool,
    is_redacted: bool,
}

impl Serialize for VersionGraphNode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let redact_content = self.is_redacted;
        VersionGraphNodeWire {
            id: self.id,
            title: (!redact_content).then(|| self.title.clone()).flatten(),
            owner_username: (!redact_content)
                .then(|| self.owner_username.clone())
                .flatten(),
            image: (!redact_content).then(|| self.image.clone()).flatten(),
            created_at: (!redact_content).then_some(self.created_at).flatten(),
            change_summary: (!redact_content)
                .then(|| self.change_summary.clone())
                .flatten(),
            is_selected_recipe: self.is_selected_recipe,
            is_current_version: self.is_current_version,
            is_deleted: self.is_deleted,
            is_redacted: self.is_redacted,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for VersionGraphNode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = VersionGraphNodeWire::deserialize(deserializer)?;
        if wire.is_redacted {
            return Ok(Self::redacted(
                wire.id,
                VersionGraphNodeState {
                    is_selected_recipe: wire.is_selected_recipe,
                    is_current_version: wire.is_current_version,
                    is_deleted: wire.is_deleted,
                },
            ));
        }

        Ok(Self {
            id: wire.id,
            title: wire.title,
            owner_username: wire.owner_username,
            image: wire.image,
            created_at: wire.created_at,
            change_summary: wire.change_summary,
            is_selected_recipe: wire.is_selected_recipe,
            is_current_version: wire.is_current_version,
            is_deleted: wire.is_deleted,
            is_redacted: false,
        })
    }
}

/// Server limits that prevented the complete connected graph from being returned.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionGraphTruncation {
    pub is_truncated: bool,
    pub node_limit_reached: bool,
    pub edge_limit_reached: bool,
    pub depth_limit_reached: bool,
    #[serde(default)]
    pub traversal_limit_reached: bool,
}

/// Normalized graph payload. Relationships refer to nodes by stable identity.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VersionGraph {
    pub nodes: Vec<VersionGraphNode>,
    pub edges: Vec<VersionGraphEdge>,
    pub truncation: VersionGraphTruncation,
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
    use chrono::TimeZone;

    #[test]
    fn version_graph_contract_round_trips_stable_nodes_and_typed_edges() {
        let source_recipe_id = Uuid::new_v4();
        let forked_recipe_id = Uuid::new_v4();
        let source_node_id = VersionGraphNodeId {
            recipe_id: source_recipe_id,
            version_number: 2,
        };
        let graph_image: RecipeImage = serde_json::from_value(serde_json::json!({
            "url": "https://example.com/soup-full.webp",
            "thumb_url": "https://example.com/soup-thumb.webp",
            "edits": [{
                "name": "rectangle:16:9",
                "version": 1,
                "crop_shape": "rectangle",
                "aspect_ratio": { "width": 16, "height": 9 },
                "crop": { "x": 0.1, "y": 0.2, "width": 0.8, "height": 0.6 },
                "transform": {
                    "rotation": 12.0,
                    "stretch_x": 1.0,
                    "stretch_y": 1.0,
                    "flip_x": false,
                    "flip_y": false
                },
                "color": {
                    "saturation": 1.1,
                    "contrast_r": 1.0,
                    "contrast_g": 1.0,
                    "contrast_b": 1.0
                }
            }]
        }))
        .unwrap();
        let graph = VersionGraph {
            nodes: vec![VersionGraphNode::visible(
                source_node_id,
                VersionGraphNodeContent {
                    title: "Tomato soup".to_string(),
                    owner_username: Some("cook".to_string()),
                    image: Some(graph_image),
                    created_at: Utc.with_ymd_and_hms(2026, 7, 21, 12, 0, 0).unwrap(),
                    change_summary: "Updated instructions".to_string(),
                },
                VersionGraphNodeState {
                    is_selected_recipe: true,
                    is_current_version: true,
                    is_deleted: false,
                },
            )],
            edges: vec![VersionGraphEdge::fork(source_recipe_id, 2, forked_recipe_id).unwrap()],
            truncation: VersionGraphTruncation {
                is_truncated: true,
                node_limit_reached: true,
                edge_limit_reached: false,
                depth_limit_reached: false,
                traversal_limit_reached: false,
            },
        };

        let serialized = serde_json::to_value(&graph).unwrap();
        let restored: VersionGraph = serde_json::from_value(serialized.clone()).unwrap();

        assert_eq!(restored, graph);
        assert_eq!(serialized["edges"][0]["kind"], "fork");
        assert_eq!(restored.nodes[0].id, source_node_id);
        assert_eq!(restored.nodes[0].image().unwrap().edits.len(), 1);
        assert_eq!(
            restored.nodes[0].thumbnail_url(),
            Some("https://example.com/soup-thumb.webp")
        );
        assert!(restored.truncation.is_truncated);
        assert!(restored.truncation.node_limit_reached);
        assert!(!restored.truncation.traversal_limit_reached);
    }

    #[test]
    fn fork_edges_connect_the_recorded_source_snapshot_to_fork_version_one() {
        let source_recipe_id = Uuid::new_v4();
        let forked_recipe_id = Uuid::new_v4();

        let edge = VersionGraphEdge::fork(source_recipe_id, 4, forked_recipe_id).unwrap();

        assert_eq!(edge.kind, VersionGraphEdgeKind::Fork);
        assert_eq!(edge.source.recipe_id, source_recipe_id);
        assert_eq!(edge.source.version_number, 4);
        assert_eq!(edge.target.recipe_id, forked_recipe_id);
        assert_eq!(edge.target.version_number, 1);
        assert!(VersionGraphEdge::fork(source_recipe_id, 0, forked_recipe_id).is_none());
        assert!(VersionGraphEdge::fork(source_recipe_id, 4, source_recipe_id).is_none());
    }

    #[test]
    fn version_edges_only_connect_consecutive_snapshots_of_one_recipe() {
        let recipe_id = Uuid::new_v4();
        let first = VersionGraphNodeId {
            recipe_id,
            version_number: 2,
        };
        let second = VersionGraphNodeId {
            recipe_id,
            version_number: 3,
        };

        assert_eq!(
            VersionGraphEdge::version(first, second).unwrap().kind,
            VersionGraphEdgeKind::Version
        );
        assert!(VersionGraphEdge::version(second, first).is_none());
        assert!(VersionGraphEdge::version(
            first,
            VersionGraphNodeId {
                recipe_id: Uuid::new_v4(),
                version_number: 3,
            },
        )
        .is_none());

        let invalid_serialized_edge = serde_json::json!({
            "source": first,
            "target": {
                "recipe_id": recipe_id,
                "version_number": 5,
            },
            "kind": "version",
        });
        assert!(serde_json::from_value::<VersionGraphEdge>(invalid_serialized_edge).is_err());
    }

    #[test]
    fn redacted_version_graph_nodes_expose_only_topology_and_state() {
        let id = VersionGraphNodeId {
            recipe_id: Uuid::new_v4(),
            version_number: 3,
        };

        let node = VersionGraphNode::redacted(
            id,
            VersionGraphNodeState {
                is_selected_recipe: false,
                is_current_version: false,
                is_deleted: true,
            },
        );

        assert_eq!(node.id, id);
        assert!(node.title.is_none());
        assert!(node.owner_username.is_none());
        assert!(node.thumbnail_url().is_none());
        assert!(node.created_at.is_none());
        assert!(node.change_summary.is_none());
        assert!(node.is_deleted);
        assert!(node.is_redacted);

        let serialized = serde_json::to_value(node).unwrap();
        assert!(serialized["title"].is_null());
        assert!(serialized["owner_username"].is_null());
        assert!(serialized["image"].is_null());
        assert!(serialized.get("thumbnail_url").is_none());
        assert!(serialized["created_at"].is_null());
        assert!(serialized["change_summary"].is_null());

        let inconsistent_redacted_node = serde_json::json!({
            "id": id,
            "title": "Private title",
            "owner_username": "private-owner",
            "thumbnail_url": "https://example.com/private.webp",
            "created_at": "2026-07-21T12:00:00Z",
            "change_summary": "Private changes",
            "is_selected_recipe": false,
            "is_current_version": false,
            "is_deleted": false,
            "is_redacted": true,
        });
        let sanitized: VersionGraphNode =
            serde_json::from_value(inconsistent_redacted_node).unwrap();
        let reserialized = serde_json::to_value(sanitized).unwrap();
        assert!(reserialized["title"].is_null());
        assert!(reserialized["owner_username"].is_null());
        assert!(reserialized["thumbnail_url"].is_null());
        assert!(reserialized["created_at"].is_null());
        assert!(reserialized["change_summary"].is_null());
    }

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
