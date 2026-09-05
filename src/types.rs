//! Shared types used across the client-server boundary.
//!
//! These types are serialized/deserialized by Dioxus `#[server]` functions
//! and must be available on both the client (wasm) and server.
#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

/// Fixed client page size for bounded immutable-version history reads.
pub const VERSION_HISTORY_PAGE_SIZE: usize = 25;

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
/// Managed images carry an asset ID; structured entries without one are
/// explicitly external and never grant managed-object authority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeImage {
    /// Stable identity for application-managed media. `None` means the URL is
    /// an explicitly external display resource and grants no storage authority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<Uuid>,
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
            asset_id: None,
            url,
            thumb_url,
            file_name: None,
            file_size: None,
            edits: vec![edit],
        }
    }

    pub fn managed(
        asset_id: Uuid,
        url: String,
        thumb_url: Option<String>,
        edit: RecipeImageEdit,
    ) -> Self {
        Self {
            asset_id: Some(asset_id),
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

/// The bounded fields needed to render one recipe card.
///
/// Collection reads use this projection directly so card pages never transport
/// a complete recipe snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeCardImage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<Uuid>,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb_url: Option<String>,
    pub edits: Vec<RecipeImageEdit>,
}

impl RecipeCardImage {
    pub fn into_recipe_image(self) -> RecipeImage {
        RecipeImage {
            asset_id: self.asset_id,
            url: self.url,
            thumb_url: self.thumb_url,
            file_name: None,
            file_size: None,
            edits: self.edits,
        }
    }
}

impl From<RecipeImage> for RecipeCardImage {
    fn from(image: RecipeImage) -> Self {
        Self {
            asset_id: image.asset_id,
            url: image.url,
            thumb_url: image.thumb_url,
            edits: image
                .edits
                .into_iter()
                .find(|edit| edit.name == "rectangle:16:9")
                .into_iter()
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeCardSummary {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub prep_time_minutes: Option<i32>,
    pub cook_time_minutes: Option<i32>,
    pub servings: Option<i32>,
    pub image: Option<RecipeCardImage>,
    pub visibility: String,
    pub author_username: String,
    pub author_avatar_url: Option<String>,
}

impl From<Recipe> for RecipeCardSummary {
    fn from(recipe: Recipe) -> Self {
        Self {
            id: recipe.id,
            title: recipe.title,
            description: recipe.description,
            prep_time_minutes: recipe.prep_time_minutes,
            cook_time_minutes: recipe.cook_time_minutes,
            servings: recipe.servings,
            image: recipe.images.into_iter().next().map(RecipeCardImage::from),
            visibility: recipe.visibility,
            author_username: recipe.author_username,
            author_avatar_url: recipe.author_avatar_url,
        }
    }
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

/// The sharing level applied to a recipe collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CollectionVisibility {
    #[default]
    Private,
    Unlisted,
    Public,
}

impl CollectionVisibility {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Unlisted => "unlisted",
            Self::Public => "public",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseCollectionVisibilityError;

impl std::fmt::Display for ParseCollectionVisibilityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("invalid collection visibility")
    }
}

impl std::error::Error for ParseCollectionVisibilityError {}

impl std::str::FromStr for CollectionVisibility {
    type Err = ParseCollectionVisibilityError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "private" => Ok(Self::Private),
            "unlisted" => Ok(Self::Unlisted),
            "public" => Ok(Self::Public),
            _ => Err(ParseCollectionVisibilityError),
        }
    }
}

/// A user-owned recipe collection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Collection {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub parent_collection_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub image: Option<RecipeImage>,
    pub visibility: CollectionVisibility,
    pub sort_order: i32,
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

/// One descending cursor page of lightweight immutable-version summaries.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RecipeVersionSummaryPage {
    pub items: Vec<RecipeVersionSummary>,
    /// Pass this value as `before_version_number` to request the next page.
    pub next_before_version_number: Option<i32>,
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

/// Directional comparison between any two visible graph snapshots.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionGraphComparison {
    pub from: VersionGraphNodeId,
    pub to: VersionGraphNodeId,
    pub changes: Vec<RecipeVersionChange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForkAttribution {
    pub original_recipe_id: Option<Uuid>,
    /// Present only when the viewer may discover the source under its current
    /// visibility, or under its captured visibility after deletion.
    pub original_title: Option<String>,
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
        if self.is_redacted {
            return Err(serde::ser::Error::custom(
                "redacted version graph nodes must be omitted before serialization",
            ));
        }
        VersionGraphNodeWire {
            id: self.id,
            title: self.title.clone(),
            owner_username: self.owner_username.clone(),
            image: self.image.clone(),
            created_at: self.created_at,
            change_summary: self.change_summary.clone(),
            is_selected_recipe: self.is_selected_recipe,
            is_current_version: self.is_current_version,
            is_deleted: self.is_deleted,
            is_redacted: false,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionGraphLineageCount {
    pub count: usize,
    pub is_capped: bool,
}

/// Aggregate topology intentionally includes inaccessible lineage branches.
/// Counts disclose only existence and size; hidden nodes and their reusable
/// identifiers are omitted from every graph page.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionGraphLineageCounts {
    pub ancestors: VersionGraphLineageCount,
    pub immediate_descendants: VersionGraphLineageCount,
    pub descendants: VersionGraphLineageCount,
}

/// The edge direction in which more graph nodes are available from an anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionGraphFrontierKind {
    OlderVersions,
    NewerVersions,
    ParentFork,
    ChildForks,
}

/// One independently expandable boundary in a progressively loaded graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VersionGraphFrontier {
    pub anchor: VersionGraphNodeId,
    pub kind: VersionGraphFrontierKind,
    pub hidden_count: usize,
    /// Random relationship key used as an opaque keyset continuation for
    /// bounded fork pages. Version-only directions leave this unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fork_cursor: Option<Uuid>,
}

/// Normalized graph payload. Relationships refer to nodes by stable identity.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VersionGraph {
    pub nodes: Vec<VersionGraphNode>,
    pub edges: Vec<VersionGraphEdge>,
    #[serde(default)]
    pub frontiers: Vec<VersionGraphFrontier>,
    pub truncation: VersionGraphTruncation,
    #[serde(default)]
    pub lineage_counts: VersionGraphLineageCounts,
}

/// Display-oriented collection data with direct child and visible recipe counts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollectionSummary {
    pub id: Uuid,
    pub parent_collection_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub image: Option<RecipeImage>,
    /// Up to four ordered recipe images used when the collection has no cover.
    pub preview_images: Vec<RecipeImage>,
    pub visibility: CollectionVisibility,
    pub sort_order: i32,
    pub child_count: i64,
    pub recipe_count: i64,
    pub updated_at: DateTime<Utc>,
}

/// Paginated public collection summaries for profile discovery.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PublicCollectionListResponse {
    pub collections: Vec<CollectionSummary>,
    pub total_count: i64,
    pub has_more: bool,
}

/// Paginated direct recipe membership for a collection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollectionRecipeList {
    pub recipes: Vec<RecipeCardSummary>,
    pub total_count: i64,
    pub has_more: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionSharePreview {
    pub total_recipe_count: i64,
    pub visible_recipe_count: i64,
    pub hidden_recipe_count: i64,
}

/// A recipe's position within one collection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeOrder {
    pub recipe_id: Uuid,
    pub sort_order: i32,
}

/// A child collection's position within its parent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollectionOrder {
    pub collection_id: Uuid,
    pub sort_order: i32,
}

/// A collection and its direct contents plus root-to-leaf navigation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollectionContents {
    pub owner_username: String,
    pub can_edit: bool,
    pub collection: CollectionSummary,
    pub breadcrumbs: Vec<CollectionSummary>,
    pub child_collections: Vec<CollectionSummary>,
    pub recipes: CollectionRecipeList,
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

// ── Search contract ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextMatchMode {
    #[default]
    AllTerms,
    AnyTerm,
    ExactPhrase,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextCriterion {
    pub text: String,
    pub mode: TextMatchMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AdvancedTextField {
    pub include: Option<TextCriterion>,
    pub exclude: Vec<TextCriterion>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntegerRange {
    pub lower_inclusive: Option<i32>,
    pub upper_exclusive: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimestampRange {
    pub lower_inclusive: Option<DateTime<Utc>>,
    pub upper_exclusive: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AdvancedRecipeSearchCriteria {
    pub title: AdvancedTextField,
    pub description: AdvancedTextField,
    pub ingredients: AdvancedTextField,
    pub instructions: AdvancedTextField,
    pub tags_any: Vec<String>,
    pub tags_excluded: Vec<String>,
    pub created_at: Option<TimestampRange>,
    pub updated_at: Option<TimestampRange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeSearchVisibility {
    #[default]
    AllVisible,
    MineOnly,
    PublicOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeSearchSort {
    #[default]
    Relevance,
    UpdatedNewest,
    UpdatedOldest,
    TitleAscending,
    TitleDescending,
    Quickest,
    Longest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RecipeSearchRequest {
    pub text: Option<String>,
    pub advanced: Option<AdvancedRecipeSearchCriteria>,
    pub visibility: RecipeSearchVisibility,
    pub tags_all: Vec<String>,
    pub total_time: Option<IntegerRange>,
    pub servings: Option<IntegerRange>,
    pub sort: RecipeSearchSort,
    pub page: u32,
    pub page_size: u32,
}

impl Default for RecipeSearchRequest {
    fn default() -> Self {
        Self {
            text: None,
            advanced: None,
            visibility: RecipeSearchVisibility::default(),
            tags_all: Vec::new(),
            total_time: None,
            servings: None,
            sort: RecipeSearchSort::default(),
            page: 1,
            page_size: crate::search::DEFAULT_SEARCH_PAGE_SIZE,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AdvancedCollectionSearchCriteria {
    pub name: AdvancedTextField,
    pub description: AdvancedTextField,
    pub updated_at: Option<TimestampRange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionSearchSort {
    #[default]
    Relevance,
    UpdatedNewest,
    UpdatedOldest,
    NameAscending,
    NameDescending,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CollectionSearchRequest {
    pub text: Option<String>,
    pub advanced: Option<AdvancedCollectionSearchCriteria>,
    pub sort: CollectionSearchSort,
    pub page: u32,
    pub page_size: u32,
}

impl Default for CollectionSearchRequest {
    fn default() -> Self {
        Self {
            text: None,
            advanced: None,
            sort: CollectionSearchSort::default(),
            page: 1,
            page_size: crate::search::DEFAULT_SEARCH_PAGE_SIZE,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AdvancedUserSearchCriteria {
    pub username: AdvancedTextField,
    pub display_name: AdvancedTextField,
    pub bio: AdvancedTextField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserSearchSort {
    #[default]
    Relevance,
    UsernameAscending,
    DisplayNameAscending,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UserSearchRequest {
    pub text: Option<String>,
    pub advanced: Option<AdvancedUserSearchCriteria>,
    pub sort: UserSearchSort,
    pub page: u32,
    pub page_size: u32,
}

impl Default for UserSearchRequest {
    fn default() -> Self {
        Self {
            text: None,
            advanced: None,
            sort: UserSearchSort::default(),
            page: 1,
            page_size: crate::search::DEFAULT_SEARCH_PAGE_SIZE,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AllSearchRequest {
    pub text: String,
    pub section_limit: u32,
}

impl Default for AllSearchRequest {
    fn default() -> Self {
        Self {
            text: String::new(),
            section_limit: crate::search::DEFAULT_ALL_SECTION_LIMIT,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResult {
    pub recipe: Recipe,
    pub relevance: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagFacetBucket {
    pub tag: String,
    pub count: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeVisibilityFacetValue {
    Private,
    Unlisted,
    Public,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibilityFacetBucket {
    pub visibility: RecipeVisibilityFacetValue,
    pub count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegerRangeFacetBucket {
    pub lower_inclusive: Option<i32>,
    pub upper_exclusive: Option<i32>,
    pub count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeSearchFacets {
    pub tags: Vec<TagFacetBucket>,
    pub visibility: Vec<VisibilityFacetBucket>,
    pub total_time: Vec<IntegerRangeFacetBucket>,
    pub servings: Vec<IntegerRangeFacetBucket>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeSearchResponse {
    pub results: Vec<SearchResult>,
    pub total_count: i64,
    pub facets: RecipeSearchFacets,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionSearchSummary {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollectionSearchResult {
    pub collection: CollectionSearchSummary,
    pub breadcrumb: Vec<String>,
    pub relevance: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollectionSearchResponse {
    pub results: Vec<CollectionSearchResult>,
    pub total_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserSearchSummary {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub public_recipe_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserSearchResult {
    pub user: UserSearchSummary,
    pub relevance: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserSearchResponse {
    pub results: Vec<UserSearchResult>,
    pub total_count: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchScope {
    All,
    Recipes,
    Collections,
    Users,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResponse {
    pub scope: SearchScope,
    pub recipes: Vec<SearchResult>,
    pub recipe_total_count: i64,
    pub collections: Vec<CollectionSearchResult>,
    pub collection_total_count: i64,
    pub users: Vec<UserSearchResult>,
    pub user_total_count: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchSuggestionKind {
    Recipe,
    Collection,
    User,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchSuggestion {
    pub kind: SearchSuggestionKind,
    pub id: Uuid,
    pub label: String,
    pub secondary_text: Option<String>,
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
            frontiers: Vec::new(),
            truncation: VersionGraphTruncation {
                is_truncated: true,
                node_limit_reached: true,
                edge_limit_reached: false,
                depth_limit_reached: false,
                traversal_limit_reached: false,
            },
            lineage_counts: VersionGraphLineageCounts {
                ancestors: VersionGraphLineageCount {
                    count: 2,
                    is_capped: false,
                },
                immediate_descendants: VersionGraphLineageCount {
                    count: 50,
                    is_capped: true,
                },
                descendants: VersionGraphLineageCount {
                    count: 50,
                    is_capped: true,
                },
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
        assert_eq!(restored.lineage_counts.ancestors.count, 2);
        assert!(!restored.lineage_counts.ancestors.is_capped);
        assert_eq!(
            serialized["lineage_counts"]["immediate_descendants"]["count"],
            50
        );
        assert!(restored.lineage_counts.immediate_descendants.is_capped);
    }

    #[test]
    fn version_graph_comparison_preserves_direction_and_cross_recipe_identity() {
        let comparison = VersionGraphComparison {
            from: VersionGraphNodeId {
                recipe_id: Uuid::from_u128(1),
                version_number: 3,
            },
            to: VersionGraphNodeId {
                recipe_id: Uuid::from_u128(2),
                version_number: 1,
            },
            changes: vec![RecipeVersionChange {
                op: "replace".to_string(),
                path: "/title".to_string(),
                old_value: Some("Before".to_string()),
                value: Some("After".to_string()),
            }],
        };

        let encoded = serde_json::to_vec(&comparison).expect("comparison is serializable");
        let restored: VersionGraphComparison =
            serde_json::from_slice(&encoded).expect("comparison crosses the client boundary");

        assert_eq!(restored, comparison);
        assert_ne!(restored.from.recipe_id, restored.to.recipe_id);
        assert_eq!(restored.from.version_number, 3);
        assert_eq!(restored.to.version_number, 1);
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
    fn redacted_version_graph_nodes_cannot_cross_the_wire_with_stable_ids() {
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

        assert!(serde_json::to_value(node).is_err());

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
        assert!(serde_json::to_value(sanitized).is_err());
    }

    #[test]
    fn collection_visibility_round_trips_as_lowercase_api_values() {
        for (visibility, serialized) in [
            (CollectionVisibility::Private, "\"private\""),
            (CollectionVisibility::Unlisted, "\"unlisted\""),
            (CollectionVisibility::Public, "\"public\""),
        ] {
            assert_eq!(serde_json::to_string(&visibility).unwrap(), serialized);
            assert_eq!(
                serde_json::from_str::<CollectionVisibility>(serialized).unwrap(),
                visibility
            );
        }

        assert_eq!(
            CollectionVisibility::default(),
            CollectionVisibility::Private
        );
        assert!(serde_json::from_str::<CollectionVisibility>("\"friends-only\"").is_err());
        assert_eq!("private".parse(), Ok(CollectionVisibility::Private));
        assert_eq!("unlisted".parse(), Ok(CollectionVisibility::Unlisted));
        assert_eq!("public".parse(), Ok(CollectionVisibility::Public));
        assert!("friends-only".parse::<CollectionVisibility>().is_err());
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
                    asset_id: None,
                    url: "https://example.com/step_1.webp".to_string(),
                    thumb_url: None,
                    file_name: None,
                    file_size: None,
                    edits: Vec::new(),
                },
                RecipeImage {
                    asset_id: None,
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

    #[test]
    fn version_graph_frontiers_round_trip_with_direction_and_hidden_count() {
        let anchor = VersionGraphNodeId {
            recipe_id: Uuid::from_u128(41),
            version_number: 3,
        };
        let graph = VersionGraph {
            frontiers: vec![VersionGraphFrontier {
                anchor,
                kind: VersionGraphFrontierKind::ChildForks,
                hidden_count: 7,
                fork_cursor: None,
            }],
            ..Default::default()
        };

        let restored: VersionGraph =
            serde_json::from_value(serde_json::to_value(&graph).unwrap()).unwrap();

        assert_eq!(restored.frontiers, graph.frontiers);
        assert_eq!(restored.frontiers[0].anchor, anchor);
        assert_eq!(restored.frontiers[0].hidden_count, 7);
    }

    #[test]
    fn version_summary_page_round_trips_cursor_and_items() {
        let page = RecipeVersionSummaryPage {
            items: vec![RecipeVersionSummary {
                version_number: 25,
                title: "Bounded history".to_string(),
                image_url: None,
                notes: None,
                created_at: Utc::now(),
                is_current: true,
                change_summary: "Title changed".to_string(),
            }],
            next_before_version_number: Some(25),
        };

        let restored: RecipeVersionSummaryPage =
            serde_json::from_value(serde_json::to_value(&page).unwrap()).unwrap();

        assert_eq!(restored, page);
    }
}
