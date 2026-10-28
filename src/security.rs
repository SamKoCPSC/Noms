//! Application-level resource limits for consumer deployments.
//!
//! These limits complement the transport ceiling and database query limits.
//! They are intentionally enforced before expensive recursive or database work.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::net::IpAddr;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::types::{
    RecipeCardSummary, RecipeEditorState, RecipeImage, RecipeImageEdit, VersionGraphFrontier,
    VersionGraphNodeId,
};
use uuid::Uuid;

pub const MAX_PAGE_SIZE: i64 = 100;
pub const MAX_PAGE_OFFSET: i64 = 10_000;
pub const MAX_PUBLIC_RECIPE_CARDS_PER_PAGE: usize = 12;
pub const MAX_PUBLIC_RECIPE_CARD_PAGE_BYTES: usize = 512 * 1024;
pub const MAX_PUBLIC_DISCOVERY_QUERY_CHARS: usize = 256;
// A 100-summary page can contain 500 images, each limited to 16 KiB.
// The aggregate ceiling also includes the summary fields and response envelope.
pub const MAX_PUBLIC_COLLECTION_PAGE_BYTES: usize = 8 * 1024 * 1024;

pub const MAX_RECIPE_TITLE_CHARS: usize = 255;
pub const MAX_RECIPE_DESCRIPTION_CHARS: usize = 10_000;
pub const MAX_RECIPE_COMMENTARY_CHARS: usize = 10_000;
pub const MAX_RECIPE_TAGS: usize = 50;
pub const MAX_RECIPE_TAG_CHARS: usize = 100;
pub const MAX_RECIPE_INGREDIENTS: usize = 200;
pub use crate::types::{
    MAX_INGREDIENT_AMOUNT_CHARS, MAX_INGREDIENT_MODIFIER_CHARS, MAX_INGREDIENT_NAME_CHARS,
    MAX_INGREDIENT_SUBSTITUTES, MAX_INGREDIENT_UNIT_CHARS,
};
pub const MAX_RECIPE_STEPS: usize = 200;
pub const MAX_RECIPE_STEP_DEPTH: usize = 8;
pub const MAX_RECIPE_STEP_TEXT_CHARS: usize = 5_000;
pub const MAX_RECIPE_EQUIPMENT: usize = 100;
pub const MAX_EQUIPMENT_NAME_CHARS: usize = 200;
pub const MAX_RECIPE_IMAGES: usize = 32;
pub const MAX_IMAGE_FILE_NAME_CHARS: usize = 255;
pub const MAX_IMAGE_EDITS: usize = 8;
pub const MAX_IMAGE_EDIT_NAME_CHARS: usize = 100;
pub const MAX_IMAGE_EDIT_METADATA_BYTES: usize = 4 * 1024;
pub const MAX_IMAGE_METADATA_BYTES: usize = 16 * 1024;
pub const MAX_RECIPE_SNAPSHOT_BYTES: usize = 1024 * 1024;
pub const MAX_PUBLIC_TAGS: usize = 500;
pub const MAX_VERSION_NOTE_CHARS: usize = 2_000;
pub const MAX_FORK_MESSAGE_CHARS: usize = 2_000;

pub const MAX_COLLECTIONS_PER_USER: usize = 500;
pub const MAX_COLLECTION_DEPTH: usize = 10;
pub const MAX_COLLECTION_CHILDREN: usize = 100;
pub const MAX_COLLECTION_MEMBERSHIPS: usize = 1_000;
pub const MAX_COLLECTION_NAME_CHARS: usize = 100;
pub const MAX_COLLECTION_DESCRIPTION_CHARS: usize = 500;
pub const MAX_COLLECTION_RECIPE_CARDS_PER_PAGE: usize = 12;
pub const MAX_COLLECTION_RECIPE_CARD_DESCRIPTION_CHARS: usize = 500;
pub const MAX_COLLECTION_RECIPE_CARD_AUTHOR_USERNAME_CHARS: usize = 30;
pub const MAX_COLLECTION_RECIPE_CARD_AUTHOR_AVATAR_URL_CHARS: usize = 2_048;
pub const MAX_COLLECTION_RECIPE_CARD_PAGE_BYTES: usize = 512 * 1024;

pub const MAX_ACTIVE_DRAFTS_PER_USER: usize = 50;
pub const MAX_VERSIONS_PER_RECIPE: usize = 250;
pub const MAX_VERSIONS_PER_USER: usize = 5_000;
pub const MAX_GRAPH_LOADED_NODE_IDS: usize = 250;
pub const MAX_VERSION_HISTORY_PAGE_SIZE: usize = crate::types::VERSION_HISTORY_PAGE_SIZE;
pub const MAX_VERSION_HISTORY_ENCODED_SNAPSHOT_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_VERSION_HISTORY_DECODED_SNAPSHOT_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_VERSION_COMPARISON_CHANGES: usize = 4_096;
pub const MAX_VERSION_COMPARISON_OUTPUT_BYTES: usize = 512 * 1024;
pub const MAX_INITIAL_GRAPH_ENCODED_SNAPSHOT_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_INITIAL_GRAPH_DECODED_SNAPSHOT_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_EXPANSION_GRAPH_ENCODED_SNAPSHOT_BYTES: usize = 12 * 1024 * 1024;
pub const MAX_EXPANSION_GRAPH_DECODED_SNAPSHOT_BYTES: usize = 12 * 1024 * 1024;
pub const MAX_CONCURRENT_EXPENSIVE_READS: usize = 4;

pub const MAX_MEDIA_BYTES_PER_USER: u64 = 512 * 1024 * 1024;
pub const MAX_MEDIA_OBJECTS_PER_USER: u64 = 2_048;
pub const DATABASE_EXECUTION_BUDGET: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipeValidationMode {
    Draft,
    Publish,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityLimitError {
    InvalidRecipePayload,
    InvalidImageMetadata,
    InvalidPagination,
    InvalidGraphExpansion,
    CollectionQuotaExceeded,
    MediaQuotaExceeded,
    RequestBudgetExceeded,
    DatabaseBudgetExceeded,
    ConcurrentReadBudgetExceeded,
    VersionComparisonOutputExceeded,
    CollectionRecipeCardPageExceeded,
}

impl std::fmt::Display for SecurityLimitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::InvalidRecipePayload => "recipe payload exceeds an application limit",
            Self::InvalidImageMetadata => "image metadata exceeds an application limit",
            Self::InvalidPagination => "pagination parameters exceed an application limit",
            Self::InvalidGraphExpansion => "graph expansion exceeds an application limit",
            Self::CollectionQuotaExceeded => "collection quota exceeded",
            Self::MediaQuotaExceeded => "media quota exceeded",
            Self::RequestBudgetExceeded => "request rate limit exceeded",
            Self::DatabaseBudgetExceeded => "database operation timed out",
            Self::ConcurrentReadBudgetExceeded => "expensive read work budget exhausted",
            Self::VersionComparisonOutputExceeded => {
                "version comparison output exceeds an application limit"
            }
            Self::CollectionRecipeCardPageExceeded => {
                "collection recipe card page exceeds an application limit"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for SecurityLimitError {}

fn chars_within(value: &str, maximum: usize) -> bool {
    value.chars().count() <= maximum
}

/// Validate the bounded, client-supplied transform metadata for one saved edit.
pub fn validate_image_edit_metadata(edit: &RecipeImageEdit) -> Result<(), SecurityLimitError> {
    let finite = [
        edit.crop.x,
        edit.crop.y,
        edit.crop.width,
        edit.crop.height,
        edit.transform.rotation,
        edit.transform.stretch_x,
        edit.transform.stretch_y,
        edit.color.saturation,
        edit.color.contrast_r,
        edit.color.contrast_g,
        edit.color.contrast_b,
    ]
    .into_iter()
    .all(f64::is_finite);
    let crop_epsilon = f64::EPSILON * 8.0;
    let crop_is_normalized = edit.crop.x >= 0.0
        && edit.crop.y >= 0.0
        && edit.crop.width > 0.0
        && edit.crop.height > 0.0
        && edit.crop.x <= 1.0
        && edit.crop.y <= 1.0
        && edit.crop.width <= 1.0
        && edit.crop.height <= 1.0
        && edit.crop.x + edit.crop.width <= 1.0 + crop_epsilon
        && edit.crop.y + edit.crop.height <= 1.0 + crop_epsilon;
    let transform_matches_editor = (-180.0..=180.0).contains(&edit.transform.rotation)
        && (0.1..=1.0).contains(&edit.transform.stretch_x)
        && (0.1..=1.0).contains(&edit.transform.stretch_y);
    let color_matches_editor = (0.0..=2.0).contains(&edit.color.saturation)
        && (0.1..=2.0).contains(&edit.color.contrast_r)
        && (0.1..=2.0).contains(&edit.color.contrast_g)
        && (0.1..=2.0).contains(&edit.color.contrast_b);
    if !chars_within(&edit.name, MAX_IMAGE_EDIT_NAME_CHARS)
        || edit.version == 0
        || edit.aspect_ratio.width == 0
        || edit.aspect_ratio.height == 0
        || !finite
        || !crop_is_normalized
        || !transform_matches_editor
        || !color_matches_editor
    {
        return Err(SecurityLimitError::InvalidImageMetadata);
    }

    let serialized =
        serde_json::to_vec(edit).map_err(|_| SecurityLimitError::InvalidImageMetadata)?;
    if serialized.len() > MAX_IMAGE_EDIT_METADATA_BYTES {
        return Err(SecurityLimitError::InvalidImageMetadata);
    }
    Ok(())
}

/// Validate one persisted recipe, collection, or avatar image metadata record.
pub fn validate_image_metadata(image: &RecipeImage) -> Result<(), SecurityLimitError> {
    if image
        .file_name
        .as_deref()
        .is_some_and(|name| !chars_within(name, MAX_IMAGE_FILE_NAME_CHARS))
        || image.edits.len() > MAX_IMAGE_EDITS
    {
        return Err(SecurityLimitError::InvalidImageMetadata);
    }
    image
        .edits
        .iter()
        .try_for_each(validate_image_edit_metadata)?;

    let serialized =
        serde_json::to_vec(image).map_err(|_| SecurityLimitError::InvalidImageMetadata)?;
    if serialized.len() > MAX_IMAGE_METADATA_BYTES {
        return Err(SecurityLimitError::InvalidImageMetadata);
    }
    Ok(())
}

/// Validate the complete serialized collection card page before it is returned.
pub fn validate_collection_recipe_card_page(
    recipes: &[RecipeCardSummary],
) -> Result<(), SecurityLimitError> {
    if recipes.len() > MAX_COLLECTION_RECIPE_CARDS_PER_PAGE {
        return Err(SecurityLimitError::CollectionRecipeCardPageExceeded);
    }

    for recipe in recipes {
        if !chars_within(&recipe.title, MAX_RECIPE_TITLE_CHARS)
            || recipe.description.as_deref().is_some_and(|description| {
                !chars_within(description, MAX_COLLECTION_RECIPE_CARD_DESCRIPTION_CHARS)
            })
            || !chars_within(
                &recipe.author_username,
                MAX_COLLECTION_RECIPE_CARD_AUTHOR_USERNAME_CHARS,
            )
            || recipe.author_avatar_url.as_deref().is_some_and(|url| {
                !chars_within(url, MAX_COLLECTION_RECIPE_CARD_AUTHOR_AVATAR_URL_CHARS)
            })
            || !matches!(
                recipe.visibility.as_str(),
                "private" | "unlisted" | "public"
            )
        {
            return Err(SecurityLimitError::CollectionRecipeCardPageExceeded);
        }

        if let Some(image) = &recipe.image {
            if image.edits.len() > 1
                || image
                    .edits
                    .first()
                    .is_some_and(|edit| edit.name != "rectangle:16:9")
                || validate_image_metadata(&image.clone().into_recipe_image()).is_err()
            {
                return Err(SecurityLimitError::CollectionRecipeCardPageExceeded);
            }
        }
    }

    let encoded = serde_json::to_vec(recipes)
        .map_err(|_| SecurityLimitError::CollectionRecipeCardPageExceeded)?;
    if encoded.len() > MAX_COLLECTION_RECIPE_CARD_PAGE_BYTES {
        return Err(SecurityLimitError::CollectionRecipeCardPageExceeded);
    }
    Ok(())
}

/// Reject excessive public discovery pages before any query starts.
pub fn validate_public_recipe_pagination(
    offset: i64,
    limit: i64,
) -> Result<(), SecurityLimitError> {
    validate_offset_pagination(offset, limit)?;
    if limit > MAX_PUBLIC_RECIPE_CARDS_PER_PAGE as i64 {
        return Err(SecurityLimitError::InvalidPagination);
    }
    Ok(())
}

/// Bound literal search text before performing public discovery database work.
pub fn validate_public_discovery_query(query: &str) -> Result<(), SecurityLimitError> {
    if !chars_within(query, MAX_PUBLIC_DISCOVERY_QUERY_CHARS) {
        return Err(SecurityLimitError::InvalidRecipePayload);
    }
    Ok(())
}

/// Include the response envelope in the public discovery byte ceiling.
pub fn validate_public_recipe_card_page(
    page: &crate::types::PublicRecipeListResponse,
) -> Result<(), SecurityLimitError> {
    validate_collection_recipe_card_page(&page.recipes)?;
    if serde_json::to_vec(page)
        .map_err(|_| SecurityLimitError::CollectionRecipeCardPageExceeded)?
        .len()
        > MAX_PUBLIC_RECIPE_CARD_PAGE_BYTES
    {
        return Err(SecurityLimitError::CollectionRecipeCardPageExceeded);
    }
    Ok(())
}

pub fn validate_public_collection_page(
    page: &crate::types::PublicCollectionListResponse,
) -> Result<(), SecurityLimitError> {
    if page.collections.len() > MAX_PAGE_SIZE as usize {
        return Err(SecurityLimitError::CollectionRecipeCardPageExceeded);
    }
    for collection in &page.collections {
        if !chars_within(&collection.name, MAX_COLLECTION_NAME_CHARS)
            || collection
                .description
                .as_deref()
                .is_some_and(|description| {
                    !chars_within(description, MAX_COLLECTION_DESCRIPTION_CHARS)
                })
            || collection.preview_images.len() > 4
        {
            return Err(SecurityLimitError::CollectionRecipeCardPageExceeded);
        }
        for image in collection
            .image
            .iter()
            .chain(collection.preview_images.iter())
        {
            validate_image_metadata(image)?;
        }
    }
    if serde_json::to_vec(page)
        .map_err(|_| SecurityLimitError::CollectionRecipeCardPageExceeded)?
        .len()
        > MAX_PUBLIC_COLLECTION_PAGE_BYTES
    {
        return Err(SecurityLimitError::CollectionRecipeCardPageExceeded);
    }
    Ok(())
}

/// Validate shape and scalar bounds iteratively before serialization or media walking.
pub fn validate_recipe_editor(
    editor: &RecipeEditorState,
    mode: RecipeValidationMode,
) -> Result<(), SecurityLimitError> {
    if !chars_within(&editor.title, MAX_RECIPE_TITLE_CHARS)
        || editor
            .description
            .as_deref()
            .is_some_and(|value| !chars_within(value, MAX_RECIPE_DESCRIPTION_CHARS))
        || editor
            .commentary
            .as_deref()
            .is_some_and(|value| !chars_within(value, MAX_RECIPE_COMMENTARY_CHARS))
        || editor.tags.len() > MAX_RECIPE_TAGS
        || editor
            .tags
            .iter()
            .any(|tag| !chars_within(tag, MAX_RECIPE_TAG_CHARS))
        || editor.ingredients.len() > MAX_RECIPE_INGREDIENTS
        || editor.ingredients.iter().any(|ingredient| {
            !ingredient_fields_within_limits(ingredient)
                || ingredient.substitutes.len() > MAX_INGREDIENT_SUBSTITUTES
                || ingredient.substitutes.iter().any(|substitute| {
                    !ingredient_fields_within_limits(substitute)
                        || !substitute.substitutes.is_empty()
                        || (mode == RecipeValidationMode::Publish
                            && substitute.name.trim().is_empty())
                })
                || (mode == RecipeValidationMode::Publish
                    && !ingredient.substitutes.is_empty()
                    && ingredient.name.trim().is_empty())
        })
        || editor.equipment.len() > MAX_RECIPE_EQUIPMENT
        || editor.instructions.len() > MAX_RECIPE_STEPS
        || editor
            .equipment
            .iter()
            .any(|item| !chars_within(&item.name, MAX_EQUIPMENT_NAME_CHARS))
        || editor.images.len() > MAX_RECIPE_IMAGES
    {
        return Err(SecurityLimitError::InvalidRecipePayload);
    }
    editor
        .images
        .iter()
        .try_for_each(validate_image_metadata)
        .map_err(|_| SecurityLimitError::InvalidRecipePayload)?;

    let mut steps = editor
        .instructions
        .iter()
        .map(|step| (step, 1_usize))
        .collect::<Vec<_>>();
    let mut step_count = 0_usize;
    let mut image_count = editor.images.len();
    let mut has_instruction = false;
    while let Some((step, depth)) = steps.pop() {
        step_count = step_count.saturating_add(1);
        image_count = image_count.saturating_add(step.photos.len());
        has_instruction |= !step.text.trim().is_empty();
        if step_count > MAX_RECIPE_STEPS
            || depth > MAX_RECIPE_STEP_DEPTH
            || image_count > MAX_RECIPE_IMAGES
            || !chars_within(&step.text, MAX_RECIPE_STEP_TEXT_CHARS)
            || step_count
                .saturating_add(steps.len())
                .saturating_add(step.sub_steps.len())
                > MAX_RECIPE_STEPS
        {
            return Err(SecurityLimitError::InvalidRecipePayload);
        }
        step.photos
            .iter()
            .try_for_each(validate_image_metadata)
            .map_err(|_| SecurityLimitError::InvalidRecipePayload)?;
        steps.extend(
            step.sub_steps
                .iter()
                .map(|child| (child, depth.saturating_add(1))),
        );
    }

    if !matches!(
        editor.visibility.as_str(),
        "private" | "unlisted" | "public"
    ) {
        return Err(SecurityLimitError::InvalidRecipePayload);
    }

    if mode == RecipeValidationMode::Publish
        && (editor.title.trim().is_empty()
            || editor.ingredients.is_empty()
            || editor
                .ingredients
                .iter()
                .all(|ingredient| ingredient.name.trim().is_empty())
            || editor.instructions.is_empty()
            || !has_instruction
            || editor.prep_time_minutes.is_some_and(|value| value < 0)
            || editor.cook_time_minutes.is_some_and(|value| value < 0)
            || editor.servings.is_some_and(|value| value <= 0))
    {
        return Err(SecurityLimitError::InvalidRecipePayload);
    }

    let serialized =
        serde_json::to_vec(editor).map_err(|_| SecurityLimitError::InvalidRecipePayload)?;
    if serialized.len() > MAX_RECIPE_SNAPSHOT_BYTES {
        return Err(SecurityLimitError::InvalidRecipePayload);
    }
    Ok(())
}

fn ingredient_fields_within_limits(ingredient: &crate::types::RecipeIngredient) -> bool {
    chars_within(&ingredient.amount, MAX_INGREDIENT_AMOUNT_CHARS)
        && chars_within(&ingredient.unit, MAX_INGREDIENT_UNIT_CHARS)
        && chars_within(&ingredient.name, MAX_INGREDIENT_NAME_CHARS)
        && chars_within(&ingredient.text_before, MAX_INGREDIENT_MODIFIER_CHARS)
        && chars_within(&ingredient.text_after, MAX_INGREDIENT_MODIFIER_CHARS)
}

/// Apply the same bounds to the canonical database snapshot representation.
pub fn validate_recipe_snapshot(
    snapshot: &crate::db::diff::RecipeSnapshot,
    visibility: &str,
    mode: RecipeValidationMode,
) -> Result<(), SecurityLimitError> {
    validate_recipe_editor(
        &RecipeEditorState {
            title: snapshot.title.clone(),
            description: snapshot.description.clone(),
            commentary: snapshot.commentary.clone(),
            prep_time_minutes: snapshot.prep_time_minutes,
            cook_time_minutes: snapshot.cook_time_minutes,
            servings: snapshot.servings,
            ingredients: snapshot.ingredients.clone(),
            instructions: snapshot.instructions.clone(),
            equipment: snapshot.equipment.clone(),
            images: snapshot.images.clone(),
            visibility: visibility.to_string(),
            tags: snapshot.tags.clone(),
        },
        mode,
    )
}

pub fn validate_offset_pagination(offset: i64, limit: i64) -> Result<(), SecurityLimitError> {
    if !(0..=MAX_PAGE_OFFSET).contains(&offset) || !(1..=MAX_PAGE_SIZE).contains(&limit) {
        return Err(SecurityLimitError::InvalidPagination);
    }
    Ok(())
}

pub fn validate_graph_expansion(
    frontier: &VersionGraphFrontier,
    loaded: &[VersionGraphNodeId],
) -> Result<(), SecurityLimitError> {
    if loaded.is_empty() || loaded.len() > MAX_GRAPH_LOADED_NODE_IDS {
        return Err(SecurityLimitError::InvalidGraphExpansion);
    }
    let unique = loaded.iter().copied().collect::<HashSet<_>>();
    if unique.len() != loaded.len()
        || !unique.contains(&frontier.anchor)
        || frontier.anchor.version_number <= 0
        || loaded.iter().any(|node| node.version_number <= 0)
        || (matches!(
            frontier.kind,
            crate::types::VersionGraphFrontierKind::OlderVersions
                | crate::types::VersionGraphFrontierKind::NewerVersions
        ) && frontier.fork_cursor.is_some())
    {
        return Err(SecurityLimitError::InvalidGraphExpansion);
    }
    Ok(())
}

pub fn validate_version_history_page(
    before_version_number: Option<i32>,
    page_size: usize,
) -> Result<(), SecurityLimitError> {
    if before_version_number.is_some_and(|version_number| version_number <= 0)
        || !(1..=MAX_VERSION_HISTORY_PAGE_SIZE).contains(&page_size)
    {
        return Err(SecurityLimitError::InvalidPagination);
    }
    Ok(())
}

pub fn validate_collection_reorder_len(length: usize) -> Result<(), SecurityLimitError> {
    (length <= MAX_COLLECTION_CHILDREN)
        .then_some(())
        .ok_or(SecurityLimitError::CollectionQuotaExceeded)
}

pub fn validate_membership_reorder_len(length: usize) -> Result<(), SecurityLimitError> {
    (length <= MAX_COLLECTION_MEMBERSHIPS)
        .then_some(())
        .ok_or(SecurityLimitError::CollectionQuotaExceeded)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MediaUsage {
    pub bytes: u64,
    pub objects: u64,
}

pub fn validate_media_quota(
    current: MediaUsage,
    incoming: MediaUsage,
) -> Result<(), SecurityLimitError> {
    let bytes = current
        .bytes
        .checked_add(incoming.bytes)
        .ok_or(SecurityLimitError::MediaQuotaExceeded)?;
    let objects = current
        .objects
        .checked_add(incoming.objects)
        .ok_or(SecurityLimitError::MediaQuotaExceeded)?;
    if bytes > MAX_MEDIA_BYTES_PER_USER || objects > MAX_MEDIA_OBJECTS_PER_USER {
        return Err(SecurityLimitError::MediaQuotaExceeded);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequestClass {
    PublicDiscovery,
    Search,
    Autocomplete,
    Graph,
    CollectionRead,
    Mutation,
    Publication,
    Upload,
}

#[derive(Debug, Clone, Copy)]
struct RequestBudget {
    maximum: usize,
    window: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RequestActor {
    User(Uuid),
    Ip(IpAddr),
}

#[derive(Debug)]
pub struct UserRequestLimiter {
    budgets: HashMap<RequestClass, RequestBudget>,
    entries: Mutex<HashMap<(RequestActor, RequestClass), Vec<Instant>>>,
}

impl UserRequestLimiter {
    #[cfg(test)]
    pub fn with_uniform_limit(maximum: usize, window: Duration) -> Self {
        Self {
            budgets: [
                RequestClass::Search,
                RequestClass::PublicDiscovery,
                RequestClass::Autocomplete,
                RequestClass::Graph,
                RequestClass::CollectionRead,
                RequestClass::Mutation,
                RequestClass::Publication,
                RequestClass::Upload,
            ]
            .into_iter()
            .map(|class| (class, RequestBudget { maximum, window }))
            .collect(),
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub fn consumer_defaults() -> Self {
        let hour = Duration::from_secs(60 * 60);
        Self {
            budgets: [
                (
                    RequestClass::Search,
                    RequestBudget {
                        maximum: 600,
                        window: hour,
                    },
                ),
                (
                    RequestClass::Autocomplete,
                    RequestBudget {
                        maximum: 1_200,
                        window: hour,
                    },
                ),
                (
                    RequestClass::Graph,
                    RequestBudget {
                        maximum: 240,
                        window: hour,
                    },
                ),
                (
                    RequestClass::CollectionRead,
                    RequestBudget {
                        maximum: 600,
                        window: hour,
                    },
                ),
                (
                    RequestClass::PublicDiscovery,
                    RequestBudget {
                        maximum: 600,
                        window: hour,
                    },
                ),
                (
                    RequestClass::Mutation,
                    RequestBudget {
                        maximum: 1_200,
                        window: hour,
                    },
                ),
                (
                    RequestClass::Publication,
                    RequestBudget {
                        maximum: 30,
                        window: hour,
                    },
                ),
                (
                    RequestClass::Upload,
                    RequestBudget {
                        maximum: 60,
                        window: hour,
                    },
                ),
            ]
            .into_iter()
            .collect(),
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub fn check(&self, user_id: Uuid, class: RequestClass) -> Result<(), SecurityLimitError> {
        self.check_at(user_id, class, Instant::now())
    }

    pub fn check_at(
        &self,
        user_id: Uuid,
        class: RequestClass,
        now: Instant,
    ) -> Result<(), SecurityLimitError> {
        self.check_actor_at(RequestActor::User(user_id), class, now)
    }

    fn check_actor_at(
        &self,
        actor: RequestActor,
        class: RequestClass,
        now: Instant,
    ) -> Result<(), SecurityLimitError> {
        let budget = self.budgets[&class];
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        const MAX_TRACKED_BUCKETS: usize = 8_192;
        let key = (actor, class);
        if entries.len() >= MAX_TRACKED_BUCKETS && !entries.contains_key(&key) {
            entries.retain(|(_, entry_class), timestamps| {
                let entry_budget = self.budgets[entry_class];
                timestamps.retain(|instant| {
                    now.saturating_duration_since(*instant) < entry_budget.window
                });
                !timestamps.is_empty()
            });
            if entries.len() >= MAX_TRACKED_BUCKETS {
                return Err(SecurityLimitError::RequestBudgetExceeded);
            }
        }
        let timestamps = entries.entry(key).or_default();
        timestamps.retain(|instant| now.saturating_duration_since(*instant) < budget.window);
        if timestamps.len() >= budget.maximum {
            return Err(SecurityLimitError::RequestBudgetExceeded);
        }
        timestamps.push(now);
        Ok(())
    }
}

static USER_REQUEST_LIMITER: OnceLock<UserRequestLimiter> = OnceLock::new();

pub fn enforce_user_request_budget(
    user_id: Uuid,
    class: RequestClass,
) -> Result<(), SecurityLimitError> {
    USER_REQUEST_LIMITER
        .get_or_init(UserRequestLimiter::consumer_defaults)
        .check(user_id, class)
}

fn resolved_anonymous_actor() -> Option<IpAddr> {
    let context = dioxus::fullstack::FullstackContext::current()?;
    let parts = context.parts_mut();
    let value = parts
        .headers
        .get(&*crate::middleware::rate_limit::RESOLVED_CLIENT_IP)?
        .to_str()
        .ok()?;
    value.parse::<IpAddr>().ok()
}

pub fn enforce_request_budget(
    authenticated_user_id: Option<Uuid>,
    class: RequestClass,
) -> Result<(), SecurityLimitError> {
    enforce_resolved_request_budget(authenticated_user_id, resolved_anonymous_actor(), class)
}

/// Raw HTTP handlers use the IP pinned by the trusted client-IP middleware.
pub fn enforce_resolved_request_budget(
    authenticated_user_id: Option<Uuid>,
    anonymous_ip: Option<IpAddr>,
    class: RequestClass,
) -> Result<(), SecurityLimitError> {
    let actor = if let Some(user_id) = authenticated_user_id {
        RequestActor::User(user_id)
    } else {
        RequestActor::Ip(anonymous_ip.ok_or(SecurityLimitError::RequestBudgetExceeded)?)
    };
    USER_REQUEST_LIMITER
        .get_or_init(UserRequestLimiter::consumer_defaults)
        .check_actor_at(actor, class, Instant::now())
}

pub async fn run_with_database_budget<T>(
    budget: Duration,
    operation: impl Future<Output = T>,
) -> Result<T, SecurityLimitError> {
    tokio::time::timeout(budget, operation)
        .await
        .map_err(|_| SecurityLimitError::DatabaseBudgetExceeded)
}

#[cfg(feature = "server")]
#[derive(Debug, Clone)]
pub struct ConcurrentReadBudget {
    permits: std::sync::Arc<tokio::sync::Semaphore>,
}

#[cfg(feature = "server")]
impl ConcurrentReadBudget {
    pub fn new(maximum_concurrent_reads: usize) -> Self {
        assert!(maximum_concurrent_reads > 0);
        Self {
            permits: std::sync::Arc::new(tokio::sync::Semaphore::new(maximum_concurrent_reads)),
        }
    }

    pub async fn run<T>(
        &self,
        operation: impl Future<Output = T>,
    ) -> Result<T, SecurityLimitError> {
        let _permit = self.permits.clone().try_acquire_owned().map_err(|_| {
            tracing::warn!(
                available_permits = self.permits.available_permits(),
                "rejected expensive read because the shared work budget is exhausted"
            );
            SecurityLimitError::ConcurrentReadBudgetExceeded
        })?;
        Ok(operation.await)
    }
}

#[cfg(feature = "server")]
static EXPENSIVE_READ_BUDGET: OnceLock<ConcurrentReadBudget> = OnceLock::new();

#[cfg(feature = "server")]
static PUBLIC_DISCOVERY_WORK_BUDGET: OnceLock<ConcurrentReadBudget> = OnceLock::new();
#[cfg(feature = "server")]
static SEARCH_WORK_BUDGET: OnceLock<ConcurrentReadBudget> = OnceLock::new();

#[cfg(feature = "server")]
pub async fn run_with_public_discovery_work_budget<T>(
    operation: impl Future<Output = T>,
) -> Result<T, SecurityLimitError> {
    PUBLIC_DISCOVERY_WORK_BUDGET
        .get_or_init(|| ConcurrentReadBudget::new(MAX_CONCURRENT_EXPENSIVE_READS))
        .run(operation)
        .await
}

#[cfg(feature = "server")]
pub async fn run_with_search_work_budget<T>(
    operation: impl Future<Output = T>,
) -> Result<T, SecurityLimitError> {
    SEARCH_WORK_BUDGET
        .get_or_init(|| ConcurrentReadBudget::new(MAX_CONCURRENT_EXPENSIVE_READS))
        .run(operation)
        .await
}

#[cfg(feature = "server")]
pub async fn run_with_expensive_read_work_budget<T>(
    operation: impl Future<Output = T>,
) -> Result<T, SecurityLimitError> {
    EXPENSIVE_READ_BUDGET
        .get_or_init(|| ConcurrentReadBudget::new(MAX_CONCURRENT_EXPENSIVE_READS))
        .run(operation)
        .await
}

#[cfg(feature = "server")]
pub async fn run_with_graph_history_work_budget<T>(
    operation: impl Future<Output = T>,
) -> Result<T, SecurityLimitError> {
    run_with_expensive_read_work_budget(operation).await
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use std::future::pending;
    use std::time::{Duration, Instant};

    use super::*;
    use crate::types::{
        RecipeEditorState, RecipeEquipment, RecipeImage, RecipeIngredient, RecipeStep,
        VersionGraphFrontier, VersionGraphFrontierKind, VersionGraphNodeId,
    };
    use uuid::Uuid;

    #[tokio::test]
    async fn discovery_and_search_admission_reject_fifth_future_and_release_on_drop() {
        for search in [false, true] {
            let mut held = Vec::new();
            for _ in 0..4 {
                let future = async move {
                    if search {
                        run_with_search_work_budget(pending::<()>()).await
                    } else {
                        run_with_public_discovery_work_budget(pending::<()>()).await
                    }
                };
                let mut future = Box::pin(future);
                let mut context = std::task::Context::from_waker(std::task::Waker::noop());
                assert!(future.as_mut().poll(&mut context).is_pending());
                held.push(future);
            }
            let polled = std::sync::atomic::AtomicBool::new(false);
            let work = async {
                polled.store(true, std::sync::atomic::Ordering::SeqCst);
            };
            let result = if search {
                run_with_search_work_budget(work).await
            } else {
                run_with_public_discovery_work_budget(work).await
            };
            assert_eq!(
                result,
                Err(SecurityLimitError::ConcurrentReadBudgetExceeded)
            );
            assert!(!polled.load(std::sync::atomic::Ordering::SeqCst));
            drop(held);
            let result = if search {
                run_with_search_work_budget(async {}).await
            } else {
                run_with_public_discovery_work_budget(async {}).await
            };
            assert_eq!(result, Ok(()));
        }
    }

    #[test]
    fn public_discovery_actor_and_page_limits_are_enforced() {
        assert!(validate_public_recipe_pagination(0, 12).is_ok());
        for (offset, limit) in [(0, 13), (0, 100), (-1, 12), (0, 0), (10001, 12)] {
            assert!(validate_public_recipe_pagination(offset, limit).is_err());
        }
        let limiter = UserRequestLimiter::consumer_defaults();
        let now = Instant::now();
        let first = RequestActor::Ip("192.0.2.1".parse().unwrap());
        let second = RequestActor::Ip("192.0.2.2".parse().unwrap());
        for _ in 0..600 {
            assert!(limiter
                .check_actor_at(first, RequestClass::PublicDiscovery, now)
                .is_ok());
        }
        assert!(limiter
            .check_actor_at(first, RequestClass::PublicDiscovery, now)
            .is_err());
        assert!(limiter
            .check_actor_at(second, RequestClass::PublicDiscovery, now)
            .is_ok());
    }

    #[test]
    fn ingredient_options_validate_substitutes_modifiers_and_publication_names() {
        let mut editor = valid_editor();
        editor.ingredients[0].optional = true;
        editor.ingredients[0].text_before = "brand".into();
        editor.ingredients[0].text_after = "finely chopped".into();
        editor.ingredients[0].substitutes = vec![RecipeIngredient::default()];
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Draft).is_ok());
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Publish).is_err());
        editor.ingredients[0].substitutes[0].name = "broth".into();
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Publish).is_ok());
        let valid = editor.clone();
        editor.ingredients[0].substitutes[0].text_after =
            "x".repeat(MAX_INGREDIENT_MODIFIER_CHARS + 1);
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Draft).is_err());
        editor = valid.clone();
        editor.ingredients[0].text_before = "x".repeat(MAX_INGREDIENT_MODIFIER_CHARS + 1);
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Draft).is_err());
        editor = valid.clone();
        editor.ingredients[0].substitutes[0].amount = "x".repeat(MAX_INGREDIENT_AMOUNT_CHARS + 1);
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Draft).is_err());
        editor = valid.clone();
        editor.ingredients[0].substitutes[0]
            .substitutes
            .push(RecipeIngredient::default());
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Draft).is_err());
        editor = valid.clone();
        editor.ingredients[0].substitutes =
            vec![valid.ingredients[0].substitutes[0].clone(); MAX_INGREDIENT_SUBSTITUTES];
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Publish).is_ok());
        editor.ingredients[0]
            .substitutes
            .push(RecipeIngredient::default());
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Draft).is_err());
    }

    fn valid_editor() -> RecipeEditorState {
        RecipeEditorState {
            title: "Weeknight soup".to_string(),
            description: Some("A practical dinner".to_string()),
            commentary: Some("Serve warm".to_string()),
            ingredients: vec![RecipeIngredient {
                vocabulary_id: None,
                amount: "1".to_string(),
                unit: "cup".to_string(),
                name: "stock".to_string(),
                ..Default::default()
            }],
            instructions: vec![RecipeStep {
                text: "Simmer".to_string(),
                sub_steps: Vec::new(),
                photos: Vec::new(),
            }],
            equipment: vec![RecipeEquipment {
                name: "pot".to_string(),
            }],
            visibility: "private".to_string(),
            ..Default::default()
        }
    }

    fn external_image(index: usize) -> RecipeImage {
        RecipeImage {
            asset_id: None,
            url: format!("https://images.example.test/{index}/full.webp"),
            thumb_url: Some(format!("https://images.example.test/{index}/thumb.webp")),
            file_name: Some(format!("image-{index}.webp")),
            file_size: Some(1_024),
            edits: Vec::new(),
        }
    }

    fn nested_steps(depth: usize) -> Vec<RecipeStep> {
        let mut current = RecipeStep {
            text: "leaf".to_string(),
            sub_steps: Vec::new(),
            photos: Vec::new(),
        };
        for _ in 1..depth {
            current = RecipeStep {
                text: "branch".to_string(),
                sub_steps: vec![current],
                photos: Vec::new(),
            };
        }
        vec![current]
    }

    #[test]
    fn recipe_payload_accepts_documented_boundaries() {
        let mut editor = valid_editor();
        editor.title = "t".repeat(MAX_RECIPE_TITLE_CHARS);
        editor.description = Some("d".repeat(MAX_RECIPE_DESCRIPTION_CHARS));
        editor.commentary = Some("c".repeat(MAX_RECIPE_COMMENTARY_CHARS));
        editor.tags = (0..MAX_RECIPE_TAGS)
            .map(|index| format!("{index:04}{}", "x".repeat(MAX_RECIPE_TAG_CHARS - 4)))
            .collect();
        editor.ingredients = (0..MAX_RECIPE_INGREDIENTS)
            .map(|_| RecipeIngredient {
                vocabulary_id: None,
                amount: "a".repeat(MAX_INGREDIENT_AMOUNT_CHARS),
                unit: "u".repeat(MAX_INGREDIENT_UNIT_CHARS),
                name: "n".repeat(MAX_INGREDIENT_NAME_CHARS),
                ..Default::default()
            })
            .collect();
        editor.instructions = nested_steps(MAX_RECIPE_STEP_DEPTH);
        editor.equipment = (0..MAX_RECIPE_EQUIPMENT)
            .map(|_| RecipeEquipment {
                name: "e".repeat(MAX_EQUIPMENT_NAME_CHARS),
            })
            .collect();
        editor.images = (0..MAX_RECIPE_IMAGES).map(external_image).collect();

        assert_eq!(
            validate_recipe_editor(&editor, RecipeValidationMode::Draft),
            Ok(())
        );
    }

    #[test]
    fn recipe_payload_rejects_each_one_over_boundary() {
        let mut editor = valid_editor();
        editor.title = "t".repeat(MAX_RECIPE_TITLE_CHARS + 1);
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Draft).is_err());

        let mut editor = valid_editor();
        editor.description = Some("d".repeat(MAX_RECIPE_DESCRIPTION_CHARS + 1));
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Draft).is_err());

        let mut editor = valid_editor();
        editor.commentary = Some("c".repeat(MAX_RECIPE_COMMENTARY_CHARS + 1));
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Draft).is_err());

        let mut editor = valid_editor();
        editor.tags = (0..=MAX_RECIPE_TAGS)
            .map(|index| format!("tag-{index}"))
            .collect();
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Draft).is_err());

        let mut editor = valid_editor();
        editor.tags = vec!["x".repeat(MAX_RECIPE_TAG_CHARS + 1)];
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Draft).is_err());

        let mut editor = valid_editor();
        editor.ingredients = (0..=MAX_RECIPE_INGREDIENTS)
            .map(|_| RecipeIngredient {
                vocabulary_id: None,
                amount: "1".to_string(),
                unit: "cup".to_string(),
                name: "stock".to_string(),
                ..Default::default()
            })
            .collect();
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Draft).is_err());

        let mut editor = valid_editor();
        editor.ingredients[0].amount = "a".repeat(MAX_INGREDIENT_AMOUNT_CHARS + 1);
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Draft).is_err());

        let mut editor = valid_editor();
        editor.ingredients[0].unit = "u".repeat(MAX_INGREDIENT_UNIT_CHARS + 1);
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Draft).is_err());

        let mut editor = valid_editor();
        editor.ingredients[0].name = "n".repeat(MAX_INGREDIENT_NAME_CHARS + 1);
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Draft).is_err());

        let mut editor = valid_editor();
        editor.instructions = (0..=MAX_RECIPE_STEPS)
            .map(|_| RecipeStep {
                text: "stir".to_string(),
                sub_steps: Vec::new(),
                photos: Vec::new(),
            })
            .collect();
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Draft).is_err());

        let mut editor = valid_editor();
        editor.instructions[0].text = "s".repeat(MAX_RECIPE_STEP_TEXT_CHARS + 1);
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Draft).is_err());

        let mut editor = valid_editor();
        editor.equipment = (0..=MAX_RECIPE_EQUIPMENT)
            .map(|_| RecipeEquipment {
                name: "spoon".to_string(),
            })
            .collect();
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Draft).is_err());

        let mut editor = valid_editor();
        editor.equipment[0].name = "e".repeat(MAX_EQUIPMENT_NAME_CHARS + 1);
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Draft).is_err());

        let mut editor = valid_editor();
        editor.images = (0..=MAX_RECIPE_IMAGES).map(external_image).collect();
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Draft).is_err());
    }

    #[test]
    fn recipe_step_depth_and_total_photos_are_bounded_iteratively() {
        let mut editor = valid_editor();
        editor.instructions = nested_steps(MAX_RECIPE_STEP_DEPTH);
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Draft).is_ok());

        editor.instructions = nested_steps(MAX_RECIPE_STEP_DEPTH + 1);
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Draft).is_err());

        let mut editor = valid_editor();
        editor.instructions[0].photos = (0..=MAX_RECIPE_IMAGES).map(external_image).collect();
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Draft).is_err());
    }

    #[test]
    fn publish_validation_keeps_required_content_checks() {
        let mut editor = valid_editor();
        editor.title = "  ".to_string();
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Publish).is_err());

        let mut editor = valid_editor();
        editor.ingredients.clear();
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Draft).is_ok());
        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Publish).is_err());
    }

    #[test]
    fn serialized_snapshot_has_a_final_application_level_ceiling() {
        let mut editor = valid_editor();
        editor.description = Some("x".repeat(MAX_RECIPE_SNAPSHOT_BYTES));

        assert!(validate_recipe_editor(&editor, RecipeValidationMode::Draft).is_err());
    }

    #[test]
    fn legacy_offset_pagination_accepts_boundary_and_rejects_extremes() {
        assert_eq!(validate_offset_pagination(0, 1), Ok(()));
        assert_eq!(
            validate_offset_pagination(MAX_PAGE_OFFSET, MAX_PAGE_SIZE),
            Ok(())
        );
        assert!(validate_offset_pagination(-1, 20).is_err());
        assert!(validate_offset_pagination(0, 0).is_err());
        assert!(validate_offset_pagination(0, MAX_PAGE_SIZE + 1).is_err());
        assert!(validate_offset_pagination(MAX_PAGE_OFFSET + 1, 20).is_err());
    }

    #[test]
    fn graph_expansion_is_rejected_before_loading_the_graph() {
        let recipe_id = Uuid::new_v4();
        let loaded = (1..=MAX_GRAPH_LOADED_NODE_IDS)
            .map(|version_number| VersionGraphNodeId {
                recipe_id,
                version_number: i32::try_from(version_number).unwrap(),
            })
            .collect::<Vec<_>>();
        let frontier = VersionGraphFrontier {
            anchor: loaded[0],
            kind: VersionGraphFrontierKind::NewerVersions,
            hidden_count: 1,
            fork_cursor: None,
        };

        assert_eq!(validate_graph_expansion(&frontier, &loaded), Ok(()));

        let mut oversized = loaded.clone();
        oversized.push(VersionGraphNodeId {
            recipe_id,
            version_number: i32::try_from(MAX_GRAPH_LOADED_NODE_IDS + 1).unwrap(),
        });
        assert!(validate_graph_expansion(&frontier, &oversized).is_err());

        let mut duplicate = loaded;
        duplicate.push(duplicate[0]);
        assert!(validate_graph_expansion(&frontier, &duplicate).is_err());
    }

    #[test]
    fn collection_and_membership_vectors_have_independent_bounds() {
        assert_eq!(
            validate_collection_reorder_len(MAX_COLLECTION_CHILDREN),
            Ok(())
        );
        assert!(validate_collection_reorder_len(MAX_COLLECTION_CHILDREN + 1).is_err());
        assert_eq!(
            validate_membership_reorder_len(MAX_COLLECTION_MEMBERSHIPS),
            Ok(())
        );
        assert!(validate_membership_reorder_len(MAX_COLLECTION_MEMBERSHIPS + 1).is_err());
    }

    #[test]
    fn media_quota_combines_namespaces_and_accepts_exact_boundary() {
        let current = MediaUsage {
            bytes: MAX_MEDIA_BYTES_PER_USER - 10,
            objects: MAX_MEDIA_OBJECTS_PER_USER - 2,
        };
        assert_eq!(
            validate_media_quota(
                current,
                MediaUsage {
                    bytes: 10,
                    objects: 2,
                }
            ),
            Ok(())
        );
        assert!(validate_media_quota(
            current,
            MediaUsage {
                bytes: 11,
                objects: 2,
            }
        )
        .is_err());
        assert!(validate_media_quota(
            current,
            MediaUsage {
                bytes: 10,
                objects: 3,
            }
        )
        .is_err());
    }

    #[test]
    fn image_metadata_has_scalar_vector_and_serialized_bounds() {
        let mut image = external_image(1);
        image.file_name = Some("f".repeat(MAX_IMAGE_FILE_NAME_CHARS));
        let mut edit = RecipeImageEdit::default_rect_16x9();
        edit.name = "e".repeat(MAX_IMAGE_EDIT_NAME_CHARS);
        image.edits = vec![edit.clone(); MAX_IMAGE_EDITS];
        assert_eq!(validate_image_metadata(&image), Ok(()));
        assert_eq!(validate_image_edit_metadata(&edit), Ok(()));

        let mut oversized_name = edit.clone();
        oversized_name.name.push('x');
        assert!(validate_image_edit_metadata(&oversized_name).is_err());

        let mut oversized_file_name = image.clone();
        oversized_file_name.file_name = Some("f".repeat(MAX_IMAGE_FILE_NAME_CHARS + 1));
        assert!(validate_image_metadata(&oversized_file_name).is_err());

        let mut excessive_edits = image.clone();
        excessive_edits.edits.push(edit);
        assert!(validate_image_metadata(&excessive_edits).is_err());

        let mut serialized_boundary = image;
        serialized_boundary.url.clear();
        let base_size = serde_json::to_vec(&serialized_boundary).unwrap().len();
        serialized_boundary.url = "u".repeat(MAX_IMAGE_METADATA_BYTES - base_size);
        assert_eq!(
            serde_json::to_vec(&serialized_boundary).unwrap().len(),
            MAX_IMAGE_METADATA_BYTES
        );
        assert_eq!(validate_image_metadata(&serialized_boundary), Ok(()));
        serialized_boundary.url.push('u');
        assert!(validate_image_metadata(&serialized_boundary).is_err());
    }

    #[test]
    fn request_budgets_are_isolated_by_user_and_operation() {
        let limiter = UserRequestLimiter::with_uniform_limit(2, Duration::from_secs(60));
        let first_user = Uuid::new_v4();
        let second_user = Uuid::new_v4();
        let now = Instant::now();

        assert!(limiter
            .check_at(first_user, RequestClass::Search, now)
            .is_ok());
        assert!(limiter
            .check_at(first_user, RequestClass::Search, now)
            .is_ok());
        assert!(limiter
            .check_at(first_user, RequestClass::Search, now)
            .is_err());
        assert!(limiter
            .check_at(second_user, RequestClass::Search, now)
            .is_ok());
        assert!(limiter
            .check_at(first_user, RequestClass::Autocomplete, now)
            .is_ok());
        assert!(limiter
            .check_at(first_user, RequestClass::CollectionRead, now)
            .is_ok());
        assert_eq!(
            UserRequestLimiter::consumer_defaults().budgets[&RequestClass::CollectionRead].maximum,
            600
        );
    }

    #[test]
    fn collection_recipe_card_pages_enforce_shape_and_unicode_bounds() {
        let card = RecipeCardSummary {
            id: Uuid::new_v4(),
            title: "Bounded card".to_string(),
            description: Some("é".repeat(MAX_COLLECTION_RECIPE_CARD_DESCRIPTION_CHARS)),
            prep_time_minutes: Some(10),
            cook_time_minutes: Some(20),
            servings: Some(4),
            image: None,
            visibility: "public".to_string(),
            author_username: "chef".to_string(),
            author_avatar_url: None,
        };
        let page = vec![card.clone(); MAX_COLLECTION_RECIPE_CARDS_PER_PAGE];
        assert_eq!(validate_collection_recipe_card_page(&page), Ok(()));

        let mut too_many = page.clone();
        too_many.push(card.clone());
        assert_eq!(
            validate_collection_recipe_card_page(&too_many),
            Err(SecurityLimitError::CollectionRecipeCardPageExceeded)
        );

        let mut oversized_description = card;
        oversized_description.description =
            Some("é".repeat(MAX_COLLECTION_RECIPE_CARD_DESCRIPTION_CHARS + 1));
        assert_eq!(
            validate_collection_recipe_card_page(&[oversized_description]),
            Err(SecurityLimitError::CollectionRecipeCardPageExceeded)
        );
    }

    #[test]
    fn anonymous_ip_and_user_actors_have_collision_free_namespaces() {
        let limiter = UserRequestLimiter::with_uniform_limit(1, Duration::from_secs(60));
        let now = Instant::now();
        let first_ip: std::net::Ipv6Addr = "2001:db8:0:1234::1".parse().unwrap();
        let second_ip: std::net::Ipv6Addr = "2001:db8:0:f234::1".parse().unwrap();
        let user_id = Uuid::from_bytes(first_ip.octets());

        assert!(limiter
            .check_actor_at(RequestActor::Ip(first_ip.into()), RequestClass::Graph, now)
            .is_ok());
        assert!(limiter
            .check_actor_at(RequestActor::Ip(second_ip.into()), RequestClass::Graph, now)
            .is_ok());
        assert!(limiter
            .check_actor_at(RequestActor::User(user_id), RequestClass::Graph, now)
            .is_ok());
    }

    #[tokio::test]
    async fn database_budget_timeout_is_a_controlled_error() {
        let result = run_with_database_budget(Duration::from_millis(1), pending::<()>()).await;
        assert_eq!(result, Err(SecurityLimitError::DatabaseBudgetExceeded));
    }

    #[test]
    fn version_history_cursor_and_page_size_are_bounded_before_database_work() {
        assert_eq!(
            validate_version_history_page(None, MAX_VERSION_HISTORY_PAGE_SIZE),
            Ok(())
        );
        assert_eq!(validate_version_history_page(Some(1), 1), Ok(()));
        assert!(validate_version_history_page(Some(0), 1).is_err());
        assert!(validate_version_history_page(None, 0).is_err());
        assert!(validate_version_history_page(None, MAX_VERSION_HISTORY_PAGE_SIZE + 1).is_err());
    }

    #[tokio::test]
    async fn expensive_read_work_rejects_excess_concurrency() {
        let budget = std::sync::Arc::new(ConcurrentReadBudget::new(1));
        let started = std::sync::Arc::new(tokio::sync::Notify::new());
        let release = std::sync::Arc::new(tokio::sync::Notify::new());
        let first_budget = budget.clone();
        let first_started = started.clone();
        let first_release = release.clone();
        let first = tokio::spawn(async move {
            first_budget
                .run(async move {
                    first_started.notify_one();
                    first_release.notified().await;
                })
                .await
        });
        started.notified().await;

        assert_eq!(
            budget.run(async {}).await,
            Err(SecurityLimitError::ConcurrentReadBudgetExceeded)
        );

        release.notify_one();
        assert_eq!(first.await.unwrap(), Ok(()));
    }
}
