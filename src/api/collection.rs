//! Collection server functions and their authorization boundary.

use dioxus::prelude::*;
use uuid::Uuid;

#[cfg(feature = "server")]
use sqlx::PgPool;

#[cfg(feature = "server")]
const COLLECTION_CONTENTS_RECIPE_LIMIT: i64 = 12;

#[cfg(feature = "server")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CollectionApiError {
    NotAuthenticated,
    InvalidCollectionId,
    InvalidRecipeId,
    InvalidName,
    NameTooLong,
    DescriptionTooLong,
    InvalidImage,
    CollectionNotFound,
    ParentNotFound,
    NameTaken,
    CollectionCycle,
    RecipeNotAccessible,
    RecipeAlreadyInCollection,
    InvalidOrder,
    InvalidPagination,
    QuotaExceeded,
    RateLimited,
    TimedOut,
    Internal,
}

#[cfg(feature = "server")]
impl std::fmt::Display for CollectionApiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::NotAuthenticated => "Not authenticated",
            Self::InvalidCollectionId => "Invalid collection ID",
            Self::InvalidRecipeId => "Invalid recipe ID",
            Self::InvalidName => "Collection name is required",
            Self::NameTooLong => "Collection name must be 100 characters or fewer",
            Self::DescriptionTooLong => "Collection description must be 500 characters or fewer",
            Self::InvalidImage => "Collection image is invalid",
            Self::CollectionNotFound => "Collection not found",
            Self::ParentNotFound => "Parent collection not found",
            Self::NameTaken => "A sibling collection already uses this name",
            Self::CollectionCycle => "A collection cannot be moved into itself or a descendant",
            Self::RecipeNotAccessible => "Recipe not accessible",
            Self::RecipeAlreadyInCollection => "Recipe is already in this collection",
            Self::InvalidOrder => "Submitted order does not match collection contents",
            Self::InvalidPagination => "Invalid collection pagination",
            Self::QuotaExceeded => "Collection quota exceeded",
            Self::RateLimited => "Mutation rate limit exceeded",
            Self::TimedOut => "Database operation timed out",
            Self::Internal => "Collection operation failed",
        };
        formatter.write_str(message)
    }
}

#[cfg(feature = "server")]
impl std::error::Error for CollectionApiError {}

#[cfg(feature = "server")]
fn require_authenticated_user(user_id: Option<Uuid>) -> Result<Uuid, CollectionApiError> {
    user_id.ok_or(CollectionApiError::NotAuthenticated)
}

#[cfg(feature = "server")]
async fn authenticated_user_id() -> Result<Uuid, CollectionApiError> {
    require_authenticated_user(crate::auth::session::extract_user_id_from_fullstack().await)
}

#[cfg(feature = "server")]
fn parse_collection_id(collection_id: &str) -> Result<Uuid, CollectionApiError> {
    Uuid::parse_str(collection_id).map_err(|_| CollectionApiError::InvalidCollectionId)
}

#[cfg(feature = "server")]
fn parse_recipe_id(recipe_id: &str) -> Result<Uuid, CollectionApiError> {
    Uuid::parse_str(recipe_id).map_err(|_| CollectionApiError::InvalidRecipeId)
}

#[cfg(feature = "server")]
fn parse_optional_collection_id(
    collection_id: Option<String>,
) -> Result<Option<Uuid>, CollectionApiError> {
    collection_id
        .as_deref()
        .map(parse_collection_id)
        .transpose()
}

#[cfg(feature = "server")]
fn parse_collection_ids(collection_ids: Vec<String>) -> Result<Vec<Uuid>, CollectionApiError> {
    collection_ids
        .iter()
        .map(|collection_id| parse_collection_id(collection_id))
        .collect()
}

#[cfg(feature = "server")]
fn parse_recipe_ids(recipe_ids: Vec<String>) -> Result<Vec<Uuid>, CollectionApiError> {
    recipe_ids
        .iter()
        .map(|recipe_id| parse_recipe_id(recipe_id))
        .collect()
}

#[cfg(feature = "server")]
fn normalize_collection_fields(
    name: String,
    description: Option<String>,
) -> Result<(String, Option<String>), CollectionApiError> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(CollectionApiError::InvalidName);
    }
    if name.chars().count() > crate::security::MAX_COLLECTION_NAME_CHARS {
        return Err(CollectionApiError::NameTooLong);
    }

    let description = description.and_then(|description| {
        let description = description.trim().to_string();
        (!description.is_empty()).then_some(description)
    });
    if description.as_ref().is_some_and(|description| {
        description.chars().count() > crate::security::MAX_COLLECTION_DESCRIPTION_CHARS
    }) {
        return Err(CollectionApiError::DescriptionTooLong);
    }

    Ok((name, description))
}

#[cfg(feature = "server")]
fn validate_collection_image_urls(
    image: Option<&crate::types::RecipeImage>,
    user_id: Uuid,
    storage: &crate::storage::StorageClient,
) -> Result<(), CollectionApiError> {
    let Some(image) = image else {
        return Ok(());
    };
    crate::security::validate_image_metadata(image)
        .map_err(|_| CollectionApiError::InvalidImage)?;
    storage
        .validate_owned_media_pair(
            &image.url,
            image.thumb_url.as_deref(),
            user_id,
            crate::storage::MediaObjectKind::CollectionFull,
        )
        .map_err(|_| CollectionApiError::InvalidImage)
}

#[cfg(feature = "server")]
fn validate_collection_image(
    image: Option<&crate::types::RecipeImage>,
    user_id: Uuid,
) -> Result<(), CollectionApiError> {
    if image.is_none() {
        return Ok(());
    }
    let storage = crate::storage::get_storage_client();
    validate_collection_image_urls(image, user_id, &storage)
}

#[cfg(feature = "server")]
fn map_db_error(error: crate::db::DbError) -> CollectionApiError {
    match error {
        crate::db::DbError::CollectionNotFound(_) => CollectionApiError::CollectionNotFound,
        crate::db::DbError::CollectionParentNotFound(_) => CollectionApiError::ParentNotFound,
        crate::db::DbError::CollectionNameTaken => CollectionApiError::NameTaken,
        crate::db::DbError::CollectionCycle => CollectionApiError::CollectionCycle,
        crate::db::DbError::RecipeNotAccessible(_) => CollectionApiError::RecipeNotAccessible,
        crate::db::DbError::RecipeAlreadyInCollection => {
            CollectionApiError::RecipeAlreadyInCollection
        }
        crate::db::DbError::InvalidCollectionOrder => CollectionApiError::InvalidOrder,
        crate::db::DbError::CollectionLimitExceeded(_) => CollectionApiError::QuotaExceeded,
        crate::db::DbError::ResourceLimitExceeded(_) => CollectionApiError::InvalidImage,
        _ => CollectionApiError::Internal,
    }
}

#[cfg(feature = "server")]
fn server_error(error: CollectionApiError) -> ServerFnError {
    ServerFnError::new(error.to_string())
}

#[cfg(feature = "server")]
fn enforce_mutation_budget(user_id: Uuid) -> Result<(), CollectionApiError> {
    crate::security::enforce_user_request_budget(user_id, crate::security::RequestClass::Mutation)
        .map_err(|_| CollectionApiError::RateLimited)
}

#[cfg(feature = "server")]
async fn with_database_budget<T>(
    operation: impl std::future::Future<Output = Result<T, CollectionApiError>>,
) -> Result<T, CollectionApiError> {
    crate::security::run_with_database_budget(crate::security::DATABASE_EXECUTION_BUDGET, operation)
        .await
        .map_err(|_| CollectionApiError::TimedOut)?
}

#[cfg(feature = "server")]
struct CollectionDiscoveryService<'pool> {
    pool: &'pool PgPool,
}

#[cfg(feature = "server")]
impl<'pool> CollectionDiscoveryService<'pool> {
    fn new(pool: &'pool PgPool) -> Self {
        Self { pool }
    }

    async fn list(
        &self,
        username: String,
        offset: i64,
        limit: i64,
    ) -> Result<crate::types::PublicCollectionListResponse, CollectionApiError> {
        crate::security::validate_offset_pagination(offset, limit)
            .map_err(|_| CollectionApiError::InvalidPagination)?;
        crate::db::get_public_collections_by_username(self.pool, &username, offset, limit)
            .await
            .map_err(map_db_error)
    }
}

#[cfg(feature = "server")]
struct CollectionReadService<'pool> {
    pool: &'pool PgPool,
    viewer_id: Option<Uuid>,
}

#[cfg(feature = "server")]
impl<'pool> CollectionReadService<'pool> {
    fn new(pool: &'pool PgPool, viewer_id: Option<Uuid>) -> Self {
        Self { pool, viewer_id }
    }

    async fn get(
        &self,
        collection_id: String,
    ) -> Result<crate::types::CollectionContents, CollectionApiError> {
        let collection_id = parse_collection_id(&collection_id)?;
        crate::db::get_collection_contents_for_viewer(
            self.pool,
            collection_id,
            self.viewer_id,
            0,
            COLLECTION_CONTENTS_RECIPE_LIMIT,
        )
        .await
        .map_err(map_db_error)?
        .ok_or(CollectionApiError::CollectionNotFound)
    }

    async fn recipes(
        &self,
        collection_id: String,
        offset: i64,
        limit: i64,
    ) -> Result<crate::types::CollectionRecipeList, CollectionApiError> {
        crate::security::validate_offset_pagination(offset, limit)
            .map_err(|_| CollectionApiError::InvalidPagination)?;
        let collection_id = parse_collection_id(&collection_id)?;
        crate::db::get_collection_contents_for_viewer(
            self.pool,
            collection_id,
            self.viewer_id,
            offset,
            limit,
        )
        .await
        .map_err(map_db_error)?
        .map(|contents| contents.recipes)
        .ok_or(CollectionApiError::CollectionNotFound)
    }
}

#[cfg(feature = "server")]
struct CollectionService<'pool> {
    pool: &'pool PgPool,
    user_id: Uuid,
}

#[cfg(feature = "server")]
impl<'pool> CollectionService<'pool> {
    fn new(pool: &'pool PgPool, user_id: Uuid) -> Self {
        Self { pool, user_id }
    }

    async fn create(
        &self,
        name: String,
        description: Option<String>,
        parent_collection_id: Option<String>,
        visibility: crate::types::CollectionVisibility,
        image: Option<crate::types::RecipeImage>,
    ) -> Result<crate::types::Collection, CollectionApiError> {
        let (name, description) = normalize_collection_fields(name, description)?;
        let parent_collection_id = parse_optional_collection_id(parent_collection_id)?;
        validate_collection_image(image.as_ref(), self.user_id)?;
        crate::db::insert_collection(
            self.pool,
            self.user_id,
            parent_collection_id,
            &name,
            description.as_deref(),
            visibility,
            image.as_ref(),
        )
        .await
        .map_err(map_db_error)
    }

    /// Load collection contents only after establishing ownership.
    async fn get(
        &self,
        collection_id: String,
    ) -> Result<crate::types::CollectionContents, CollectionApiError> {
        let collection_id = parse_collection_id(&collection_id)?;
        let breadcrumbs =
            crate::db::get_collection_breadcrumbs(self.pool, collection_id, self.user_id)
                .await
                .map_err(map_db_error)?;
        let collection = breadcrumbs
            .last()
            .cloned()
            .ok_or(CollectionApiError::CollectionNotFound)?;
        let child_collections =
            crate::db::get_child_collections(self.pool, collection_id, self.user_id)
                .await
                .map_err(map_db_error)?;
        let recipes = crate::db::get_collection_recipes(
            self.pool,
            collection_id,
            self.user_id,
            0,
            COLLECTION_CONTENTS_RECIPE_LIMIT,
        )
        .await
        .map_err(map_db_error)?;
        let owner_username = crate::db::get_user_by_id(self.pool, self.user_id)
            .await
            .map_err(map_db_error)?
            .map(|owner| owner.username)
            .ok_or(CollectionApiError::Internal)?;

        Ok(crate::types::CollectionContents {
            owner_username,
            can_edit: true,
            collection,
            breadcrumbs,
            child_collections,
            recipes,
        })
    }

    async fn roots(&self) -> Result<Vec<crate::types::CollectionSummary>, CollectionApiError> {
        crate::db::get_root_collections(self.pool, self.user_id)
            .await
            .map_err(map_db_error)
    }

    async fn children(
        &self,
        parent_collection_id: String,
    ) -> Result<Vec<crate::types::CollectionSummary>, CollectionApiError> {
        let parent_collection_id = parse_collection_id(&parent_collection_id)?;
        if crate::db::get_collection_by_id_and_owner(self.pool, parent_collection_id, self.user_id)
            .await
            .map_err(map_db_error)?
            .is_none()
        {
            return Err(CollectionApiError::CollectionNotFound);
        }
        crate::db::get_child_collections(self.pool, parent_collection_id, self.user_id)
            .await
            .map_err(map_db_error)
    }

    #[cfg(test)]
    async fn all(&self) -> Result<Vec<crate::types::CollectionSummary>, CollectionApiError> {
        crate::db::get_all_collections_by_owner(self.pool, self.user_id)
            .await
            .map_err(map_db_error)
    }

    async fn all_page(
        &self,
        offset: i64,
        limit: i64,
    ) -> Result<crate::types::PublicCollectionListResponse, CollectionApiError> {
        crate::security::validate_offset_pagination(offset, limit)
            .map_err(|_| CollectionApiError::InvalidPagination)?;
        crate::db::get_collections_by_owner_paginated(self.pool, self.user_id, offset, limit)
            .await
            .map_err(map_db_error)
    }

    async fn move_to(
        &self,
        collection_id: String,
        new_parent_collection_id: Option<String>,
    ) -> Result<crate::types::Collection, CollectionApiError> {
        let collection_id = parse_collection_id(&collection_id)?;
        let new_parent_collection_id = parse_optional_collection_id(new_parent_collection_id)?;
        crate::db::move_collection(
            self.pool,
            collection_id,
            self.user_id,
            new_parent_collection_id,
        )
        .await
        .map_err(map_db_error)
    }

    async fn reorder_children(
        &self,
        parent_collection_id: Option<String>,
        ordered_collection_ids: Vec<String>,
    ) -> Result<(), CollectionApiError> {
        crate::security::validate_collection_reorder_len(ordered_collection_ids.len())
            .map_err(|_| CollectionApiError::QuotaExceeded)?;
        let parent_collection_id = parse_optional_collection_id(parent_collection_id)?;
        let ordered_collection_ids = parse_collection_ids(ordered_collection_ids)?;
        crate::db::reorder_child_collections(
            self.pool,
            self.user_id,
            parent_collection_id,
            &ordered_collection_ids,
        )
        .await
        .map_err(map_db_error)
    }

    async fn update(
        &self,
        collection_id: String,
        name: String,
        description: Option<String>,
        image: Option<crate::types::RecipeImage>,
    ) -> Result<crate::types::Collection, CollectionApiError> {
        let collection_id = parse_collection_id(&collection_id)?;
        let (name, description) = normalize_collection_fields(name, description)?;
        validate_collection_image(image.as_ref(), self.user_id)?;
        crate::db::update_collection(
            self.pool,
            collection_id,
            self.user_id,
            &name,
            description.as_deref(),
            image.as_ref(),
        )
        .await
        .map_err(map_db_error)
    }

    async fn update_visibility(
        &self,
        collection_id: String,
        visibility: crate::types::CollectionVisibility,
    ) -> Result<crate::types::Collection, CollectionApiError> {
        let collection_id = parse_collection_id(&collection_id)?;
        crate::db::update_collection_visibility(self.pool, collection_id, self.user_id, visibility)
            .await
            .map_err(map_db_error)
    }

    async fn share_preview(
        &self,
        collection_id: String,
        target_visibility: crate::types::CollectionVisibility,
    ) -> Result<crate::types::CollectionSharePreview, CollectionApiError> {
        let collection_id = parse_collection_id(&collection_id)?;
        crate::db::get_collection_share_preview(
            self.pool,
            collection_id,
            self.user_id,
            target_visibility,
        )
        .await
        .map_err(map_db_error)
    }

    async fn delete(&self, collection_id: String) -> Result<(), CollectionApiError> {
        let collection_id = parse_collection_id(&collection_id)?;
        crate::db::delete_collection(self.pool, collection_id, self.user_id)
            .await
            .map_err(map_db_error)
    }

    async fn add_recipe(
        &self,
        collection_id: String,
        recipe_id: String,
    ) -> Result<(), CollectionApiError> {
        let collection_id = parse_collection_id(&collection_id)?;
        let recipe_id = parse_recipe_id(&recipe_id)?;
        crate::db::add_recipe_to_collection(self.pool, collection_id, self.user_id, recipe_id)
            .await
            .map(|_| ())
            .map_err(map_db_error)
    }

    async fn remove_recipe(
        &self,
        collection_id: String,
        recipe_id: String,
    ) -> Result<(), CollectionApiError> {
        let collection_id = parse_collection_id(&collection_id)?;
        let recipe_id = parse_recipe_id(&recipe_id)?;
        crate::db::remove_recipe_from_collection(self.pool, collection_id, self.user_id, recipe_id)
            .await
            .map_err(map_db_error)
    }

    async fn reorder_recipes(
        &self,
        collection_id: String,
        ordered_recipe_ids: Vec<String>,
    ) -> Result<(), CollectionApiError> {
        crate::security::validate_membership_reorder_len(ordered_recipe_ids.len())
            .map_err(|_| CollectionApiError::QuotaExceeded)?;
        let collection_id = parse_collection_id(&collection_id)?;
        let ordered_recipe_ids = parse_recipe_ids(ordered_recipe_ids)?;
        crate::db::reorder_collection_recipes(
            self.pool,
            collection_id,
            self.user_id,
            &ordered_recipe_ids,
        )
        .await
        .map_err(map_db_error)
    }

    async fn recipe_collections(&self, recipe_id: String) -> Result<Vec<Uuid>, CollectionApiError> {
        let recipe_id = parse_recipe_id(&recipe_id)?;
        crate::db::get_recipe_collections(self.pool, recipe_id, self.user_id)
            .await
            .map_err(map_db_error)
    }
}

/// Create a root or nested collection for the authenticated user.
#[server]
pub async fn create_collection(
    name: String,
    description: Option<String>,
    parent_collection_id: Option<String>,
    visibility: crate::types::CollectionVisibility,
    image: Option<crate::types::RecipeImage>,
) -> Result<crate::types::Collection, ServerFnError> {
    let user_id = authenticated_user_id().await.map_err(server_error)?;
    enforce_mutation_budget(user_id).map_err(server_error)?;
    let pool = crate::db::get_pool();
    with_database_budget(CollectionService::new(&pool, user_id).create(
        name,
        description,
        parent_collection_id,
        visibility,
        image,
    ))
    .await
    .map_err(server_error)
}

/// Return a viewer-safe collection with breadcrumbs, children, and the first recipe page.
#[server]
pub async fn get_collection(
    collection_id: String,
) -> Result<crate::types::CollectionContents, ServerFnError> {
    let viewer_id = crate::auth::session::extract_user_id_from_fullstack().await;
    let pool = crate::db::get_pool();
    with_database_budget(CollectionReadService::new(&pool, viewer_id).get(collection_id))
        .await
        .map_err(server_error)
}

/// List public collections for a user profile without requiring a session.
#[server]
pub async fn get_user_public_collections(
    username: String,
    offset: i64,
    limit: i64,
) -> Result<crate::types::PublicCollectionListResponse, ServerFnError> {
    let pool = crate::db::get_pool();
    with_database_budget(CollectionDiscoveryService::new(&pool).list(username, offset, limit))
        .await
        .map_err(server_error)
}

/// Return an owned collection for authenticated management screens.
#[server]
pub async fn get_owned_collection(
    collection_id: String,
) -> Result<crate::types::CollectionContents, ServerFnError> {
    let user_id = authenticated_user_id().await.map_err(server_error)?;
    let pool = crate::db::get_pool();
    with_database_budget(CollectionService::new(&pool, user_id).get(collection_id))
        .await
        .map_err(server_error)
}

/// List root collections owned by the authenticated user.
#[server]
pub async fn list_root_collections() -> Result<Vec<crate::types::CollectionSummary>, ServerFnError>
{
    let user_id = authenticated_user_id().await.map_err(server_error)?;
    let pool = crate::db::get_pool();
    with_database_budget(CollectionService::new(&pool, user_id).roots())
        .await
        .map_err(server_error)
}

/// List direct children of an owned collection.
#[server]
pub async fn list_collection_children(
    parent_collection_id: String,
) -> Result<Vec<crate::types::CollectionSummary>, ServerFnError> {
    let user_id = authenticated_user_id().await.map_err(server_error)?;
    let pool = crate::db::get_pool();
    with_database_budget(CollectionService::new(&pool, user_id).children(parent_collection_id))
        .await
        .map_err(server_error)
}

/// List one bounded page of collections owned by the authenticated user.
#[server]
pub async fn list_my_collections(
    offset: i64,
    limit: i64,
) -> Result<crate::types::PublicCollectionListResponse, ServerFnError> {
    crate::security::validate_offset_pagination(offset, limit)
        .map_err(|_| server_error(CollectionApiError::InvalidPagination))?;
    let user_id = authenticated_user_id().await.map_err(server_error)?;
    let pool = crate::db::get_pool();
    with_database_budget(CollectionService::new(&pool, user_id).all_page(offset, limit))
        .await
        .map_err(server_error)
}

/// Load every owned collection through bounded server pages for pickers and hierarchy editors.
pub async fn load_all_my_collections() -> Result<Vec<crate::types::CollectionSummary>, ServerFnError>
{
    const PAGE_SIZE: i64 = 100;
    const MAX_PAGES: usize = 5;
    let mut collections = Vec::new();
    for page_index in 0..MAX_PAGES {
        let offset = i64::try_from(page_index).unwrap_or_default() * PAGE_SIZE;
        let page = list_my_collections(offset, PAGE_SIZE).await?;
        collections.extend(page.collections);
        if !page.has_more {
            return Ok(collections);
        }
    }
    Ok(collections)
}

/// Move an owned collection to another owned parent or to the root.
#[server]
pub async fn move_collection(
    collection_id: String,
    new_parent_collection_id: Option<String>,
) -> Result<crate::types::Collection, ServerFnError> {
    let user_id = authenticated_user_id().await.map_err(server_error)?;
    enforce_mutation_budget(user_id).map_err(server_error)?;
    let pool = crate::db::get_pool();
    with_database_budget(
        CollectionService::new(&pool, user_id).move_to(collection_id, new_parent_collection_id),
    )
    .await
    .map_err(server_error)
}

/// Normalize the complete direct-child order for a parent or the root.
#[server]
pub async fn reorder_child_collections(
    parent_collection_id: Option<String>,
    ordered_collection_ids: Vec<String>,
) -> Result<(), ServerFnError> {
    let user_id = authenticated_user_id().await.map_err(server_error)?;
    enforce_mutation_budget(user_id).map_err(server_error)?;
    crate::security::validate_collection_reorder_len(ordered_collection_ids.len())
        .map_err(|_| server_error(CollectionApiError::QuotaExceeded))?;
    let pool = crate::db::get_pool();
    with_database_budget(
        CollectionService::new(&pool, user_id)
            .reorder_children(parent_collection_id, ordered_collection_ids),
    )
    .await
    .map_err(server_error)
}

/// Rename or change the description of an owned collection.
#[server]
pub async fn update_collection(
    collection_id: String,
    name: String,
    description: Option<String>,
    image: Option<crate::types::RecipeImage>,
) -> Result<crate::types::Collection, ServerFnError> {
    let user_id = authenticated_user_id().await.map_err(server_error)?;
    enforce_mutation_budget(user_id).map_err(server_error)?;
    let pool = crate::db::get_pool();
    with_database_budget(CollectionService::new(&pool, user_id).update(
        collection_id,
        name,
        description,
        image,
    ))
    .await
    .map_err(server_error)
}

/// Change an owned collection's visibility.
#[server]
pub async fn update_collection_visibility(
    collection_id: String,
    visibility: crate::types::CollectionVisibility,
) -> Result<crate::types::Collection, ServerFnError> {
    let user_id = authenticated_user_id().await.map_err(server_error)?;
    enforce_mutation_budget(user_id).map_err(server_error)?;
    let pool = crate::db::get_pool();
    with_database_budget(
        CollectionService::new(&pool, user_id).update_visibility(collection_id, visibility),
    )
    .await
    .map_err(server_error)
}

/// Preview recipe visibility before sharing an owned collection.
#[server]
pub async fn get_collection_share_preview(
    collection_id: String,
    target_visibility: crate::types::CollectionVisibility,
) -> Result<crate::types::CollectionSharePreview, ServerFnError> {
    let user_id = authenticated_user_id().await.map_err(server_error)?;
    let pool = crate::db::get_pool();
    with_database_budget(
        CollectionService::new(&pool, user_id).share_preview(collection_id, target_visibility),
    )
    .await
    .map_err(server_error)
}

/// Delete an owned collection and its descendants without deleting recipes.
#[server]
pub async fn delete_collection(collection_id: String) -> Result<(), ServerFnError> {
    let user_id = authenticated_user_id().await.map_err(server_error)?;
    enforce_mutation_budget(user_id).map_err(server_error)?;
    let pool = crate::db::get_pool();
    with_database_budget(CollectionService::new(&pool, user_id).delete(collection_id))
        .await
        .map_err(server_error)
}

/// Add an accessible published recipe to an owned collection.
#[server]
pub async fn add_recipe_to_collection(
    collection_id: String,
    recipe_id: String,
) -> Result<(), ServerFnError> {
    let user_id = authenticated_user_id().await.map_err(server_error)?;
    enforce_mutation_budget(user_id).map_err(server_error)?;
    let pool = crate::db::get_pool();
    with_database_budget(
        CollectionService::new(&pool, user_id).add_recipe(collection_id, recipe_id),
    )
    .await
    .map_err(server_error)
}

/// Remove a recipe membership from an owned collection.
#[server]
pub async fn remove_recipe_from_collection(
    collection_id: String,
    recipe_id: String,
) -> Result<(), ServerFnError> {
    let user_id = authenticated_user_id().await.map_err(server_error)?;
    enforce_mutation_budget(user_id).map_err(server_error)?;
    let pool = crate::db::get_pool();
    with_database_budget(
        CollectionService::new(&pool, user_id).remove_recipe(collection_id, recipe_id),
    )
    .await
    .map_err(server_error)
}

/// Return a viewer-safe page of direct recipe members from a collection.
#[server]
pub async fn get_collection_recipes(
    collection_id: String,
    offset: i64,
    limit: i64,
) -> Result<crate::types::CollectionRecipeList, ServerFnError> {
    crate::security::validate_offset_pagination(offset, limit)
        .map_err(|_| server_error(CollectionApiError::InvalidPagination))?;
    let viewer_id = crate::auth::session::extract_user_id_from_fullstack().await;
    let pool = crate::db::get_pool();
    with_database_budget(CollectionReadService::new(&pool, viewer_id).recipes(
        collection_id,
        offset,
        limit,
    ))
    .await
    .map_err(server_error)
}

/// Normalize the complete direct recipe order for an owned collection.
#[server]
pub async fn reorder_collection_recipes(
    collection_id: String,
    ordered_recipe_ids: Vec<String>,
) -> Result<(), ServerFnError> {
    let user_id = authenticated_user_id().await.map_err(server_error)?;
    enforce_mutation_budget(user_id).map_err(server_error)?;
    crate::security::validate_membership_reorder_len(ordered_recipe_ids.len())
        .map_err(|_| server_error(CollectionApiError::QuotaExceeded))?;
    let pool = crate::db::get_pool();
    with_database_budget(
        CollectionService::new(&pool, user_id).reorder_recipes(collection_id, ordered_recipe_ids),
    )
    .await
    .map_err(server_error)
}

/// Return IDs of owned collections containing an accessible recipe.
#[server]
pub async fn get_recipe_collections(recipe_id: String) -> Result<Vec<Uuid>, ServerFnError> {
    let user_id = authenticated_user_id().await.map_err(server_error)?;
    let pool = crate::db::get_pool();
    with_database_budget(CollectionService::new(&pool, user_id).recipe_collections(recipe_id))
        .await
        .map_err(server_error)
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;
    use crate::test_utils;

    #[test]
    fn collection_creation_uses_canonical_names_and_requires_visibility() {
        let api_source = include_str!("collection.rs");
        let db_source = include_str!("../db/mod.rs");

        assert!(api_source.contains(concat!("pub async fn create_", "collection(")));
        assert!(api_source.contains(concat!("async fn create", "(")));
        assert!(db_source.contains(concat!("pub async fn insert_", "collection(")));
        assert!(!api_source.contains(concat!("create", "_with_visibility")));
        assert!(!db_source.contains(concat!("insert_collection", "_with_visibility")));
        assert!(!api_source.contains(concat!(
            "Retained as the owner-only compatibility ",
            "facade for internal callers"
        )));
    }

    #[test]
    fn collection_contract_includes_an_optional_cover_image() {
        let api_source = include_str!("collection.rs");
        let db_source = include_str!("../db/mod.rs");
        let type_source = include_str!("../types.rs");
        let schema_source = include_str!("../../migrations/schema.sql");

        assert!(api_source.contains("image: Option<crate::types::RecipeImage>"));
        assert!(db_source.contains("image: Option<&RecipeImage>"));
        assert!(type_source.contains("pub image: Option<RecipeImage>"));
        assert!(schema_source.contains("image JSONB"));
    }

    #[test]
    fn collection_cover_urls_must_belong_to_the_authenticated_user() {
        let owner_id = Uuid::new_v4();
        let other_id = Uuid::new_v4();
        let upload_id = Uuid::new_v4();
        let public_url = "https://media.example.test/noms-media";
        let storage = crate::storage::StorageClient::for_test_with_endpoint_and_public_url(
            "http://127.0.0.1:9",
            public_url,
        );
        let image = crate::types::RecipeImage::new(
            format!("{public_url}/collections/{owner_id}/{upload_id}_full.webp"),
            Some(format!(
                "{public_url}/collections/{owner_id}/{upload_id}_thumb.webp"
            )),
            crate::types::RecipeImageEdit::default_rect_16x9(),
        );

        assert_eq!(
            validate_collection_image_urls(Some(&image), owner_id, &storage),
            Ok(())
        );
        assert_eq!(
            validate_collection_image_urls(Some(&image), other_id, &storage),
            Err(CollectionApiError::InvalidImage)
        );

        let external = crate::types::RecipeImage {
            url: "https://attacker.example/cover_full.webp".to_string(),
            ..image.clone()
        };
        assert_eq!(
            validate_collection_image_urls(Some(&external), owner_id, &storage),
            Err(CollectionApiError::InvalidImage)
        );

        for path in [
            format!("{public_url}/recipes/{owner_id}/{upload_id}_full.webp"),
            format!("{public_url}/avatars/{owner_id}/{upload_id}.webp"),
            format!("{public_url}/collections/{owner_id}/../{other_id}/{upload_id}_full.webp"),
            format!("{public_url}/collections/{owner_id}/%2e%2e/{other_id}/{upload_id}_full.webp"),
            format!("{public_url}/collections/{owner_id}/not-a-uuid_full.webp"),
            format!("{public_url}/collections/{owner_id}//{upload_id}_full.webp"),
            format!("{public_url}/collections/{owner_id}/{upload_id}_full.webp?download=1"),
        ] {
            let noncanonical = crate::types::RecipeImage {
                url: path,
                thumb_url: None,
                ..image.clone()
            };
            assert_eq!(
                validate_collection_image_urls(Some(&noncanonical), owner_id, &storage),
                Err(CollectionApiError::InvalidImage)
            );
        }

        let mismatched_thumbnail = crate::types::RecipeImage {
            thumb_url: Some(format!(
                "{public_url}/collections/{owner_id}/{}_thumb.webp",
                Uuid::new_v4()
            )),
            ..image
        };
        assert_eq!(
            validate_collection_image_urls(Some(&mismatched_thumbnail), owner_id, &storage),
            Err(CollectionApiError::InvalidImage)
        );
    }

    #[tokio::test]
    async fn authentication_and_inputs_return_controlled_errors() {
        assert_eq!(
            require_authenticated_user(None),
            Err(CollectionApiError::NotAuthenticated)
        );
        let unauthenticated = create_collection(
            "Meals".to_string(),
            None,
            None,
            crate::types::CollectionVisibility::Private,
            None,
        )
        .await
        .unwrap_err();
        assert!(unauthenticated.to_string().contains("Not authenticated"));
        assert_eq!(
            parse_collection_id("not-a-uuid"),
            Err(CollectionApiError::InvalidCollectionId)
        );
        assert_eq!(
            parse_recipe_id("not-a-uuid"),
            Err(CollectionApiError::InvalidRecipeId)
        );
        assert_eq!(
            normalize_collection_fields("   ".to_string(), None),
            Err(CollectionApiError::InvalidName)
        );
        assert_eq!(
            normalize_collection_fields("x".repeat(101), None),
            Err(CollectionApiError::NameTooLong)
        );
        assert_eq!(
            normalize_collection_fields("Valid".to_string(), Some("x".repeat(501))),
            Err(CollectionApiError::DescriptionTooLong)
        );
    }

    #[tokio::test]
    async fn service_creates_explicit_visibility_and_keeps_updates_owner_scoped() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        let other_owner = test_utils::insert_test_user(&pool).await;
        let service = CollectionService::new(&pool, owner.id);
        let other_service = CollectionService::new(&pool, other_owner.id);

        let private = service
            .create(
                "Private".to_string(),
                None,
                None,
                crate::types::CollectionVisibility::Private,
                None,
            )
            .await
            .unwrap();
        assert_eq!(
            private.visibility,
            crate::types::CollectionVisibility::Private
        );

        let public = service
            .create(
                "Shared".to_string(),
                None,
                None,
                crate::types::CollectionVisibility::Public,
                None,
            )
            .await
            .unwrap();
        assert_eq!(
            public.visibility,
            crate::types::CollectionVisibility::Public
        );

        let updated = service
            .update_visibility(
                public.id.to_string(),
                crate::types::CollectionVisibility::Unlisted,
            )
            .await
            .unwrap();
        assert_eq!(
            updated.visibility,
            crate::types::CollectionVisibility::Unlisted
        );
        assert_eq!(
            other_service
                .update_visibility(
                    public.id.to_string(),
                    crate::types::CollectionVisibility::Private,
                )
                .await,
            Err(CollectionApiError::CollectionNotFound)
        );
    }

    #[tokio::test]
    async fn read_service_uses_optional_viewer_without_weakening_owner_mutations() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        let viewer = test_utils::insert_test_user(&pool).await;
        let owner_service = CollectionService::new(&pool, owner.id);
        let private = owner_service
            .create(
                "Private".to_string(),
                None,
                None,
                crate::types::CollectionVisibility::Private,
                None,
            )
            .await
            .unwrap();
        let public = owner_service
            .create(
                "Public".to_string(),
                None,
                None,
                crate::types::CollectionVisibility::Public,
                None,
            )
            .await
            .unwrap();

        let anonymous = CollectionReadService::new(&pool, None);
        assert_eq!(
            anonymous.get(private.id.to_string()).await,
            Err(CollectionApiError::CollectionNotFound)
        );
        let anonymous_public = anonymous.get(public.id.to_string()).await.unwrap();
        assert!(!anonymous_public.can_edit);
        assert_eq!(anonymous_public.owner_username, owner.username);

        let owner_reader = CollectionReadService::new(&pool, Some(owner.id));
        assert!(
            owner_reader
                .get(private.id.to_string())
                .await
                .unwrap()
                .can_edit
        );

        let other_reader = CollectionReadService::new(&pool, Some(viewer.id));
        assert!(
            !other_reader
                .get(public.id.to_string())
                .await
                .unwrap()
                .can_edit
        );
        assert_eq!(
            CollectionService::new(&pool, viewer.id)
                .get(public.id.to_string())
                .await,
            Err(CollectionApiError::CollectionNotFound)
        );
        assert_eq!(
            other_reader.recipes(public.id.to_string(), -1, 20).await,
            Err(CollectionApiError::InvalidPagination)
        );
        assert_eq!(
            CollectionService::new(&pool, viewer.id)
                .update_visibility(
                    public.id.to_string(),
                    crate::types::CollectionVisibility::Private,
                )
                .await,
            Err(CollectionApiError::CollectionNotFound)
        );
    }

    #[tokio::test]
    async fn share_preview_service_is_owner_only() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        let other_user = test_utils::insert_test_user(&pool).await;
        let collection = CollectionService::new(&pool, owner.id)
            .create(
                "Preview".to_string(),
                None,
                None,
                crate::types::CollectionVisibility::Private,
                None,
            )
            .await
            .unwrap();

        let preview = CollectionService::new(&pool, owner.id)
            .share_preview(
                collection.id.to_string(),
                crate::types::CollectionVisibility::Public,
            )
            .await
            .unwrap();
        assert_eq!(preview.total_recipe_count, 0);
        assert_eq!(preview.hidden_recipe_count, 0);
        assert_eq!(
            CollectionService::new(&pool, other_user.id)
                .share_preview(
                    collection.id.to_string(),
                    crate::types::CollectionVisibility::Public,
                )
                .await,
            Err(CollectionApiError::CollectionNotFound)
        );
    }

    #[tokio::test]
    async fn public_collection_discovery_is_paginated_and_session_independent() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        CollectionService::new(&pool, owner.id)
            .create(
                "Public".to_string(),
                None,
                None,
                crate::types::CollectionVisibility::Public,
                None,
            )
            .await
            .unwrap();
        CollectionService::new(&pool, owner.id)
            .create(
                "Unlisted".to_string(),
                None,
                None,
                crate::types::CollectionVisibility::Unlisted,
                None,
            )
            .await
            .unwrap();

        let discovery = CollectionDiscoveryService::new(&pool);
        let listed = discovery.list(owner.username.clone(), 0, 20).await.unwrap();
        assert_eq!(listed.total_count, 1);
        assert_eq!(listed.collections[0].name, "Public");
        assert_eq!(
            discovery.list(owner.username, -1, 20).await,
            Err(CollectionApiError::InvalidPagination)
        );
    }

    #[tokio::test]
    async fn service_enforces_owner_scope_hierarchy_and_names() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        let other_owner = test_utils::insert_test_user(&pool).await;
        let service = CollectionService::new(&pool, owner.id);
        let other_service = CollectionService::new(&pool, other_owner.id);

        let root = service
            .create(
                "  Meals  ".to_string(),
                Some("  Family meals  ".to_string()),
                None,
                crate::types::CollectionVisibility::Private,
                None,
            )
            .await
            .unwrap();
        assert_eq!(root.name, "Meals");
        assert_eq!(root.description.as_deref(), Some("Family meals"));
        assert_eq!(root.sort_order, 0);

        let child = service
            .create(
                "Weeknight".to_string(),
                None,
                Some(root.id.to_string()),
                crate::types::CollectionVisibility::Private,
                None,
            )
            .await
            .unwrap();
        assert_eq!(child.parent_collection_id, Some(root.id));
        assert_eq!(child.sort_order, 0);
        let grandchild = service
            .create(
                "Fast".to_string(),
                None,
                Some(child.id.to_string()),
                crate::types::CollectionVisibility::Private,
                None,
            )
            .await
            .unwrap();
        assert_eq!(grandchild.parent_collection_id, Some(child.id));
        assert_eq!(grandchild.sort_order, 0);
        let weekend = service
            .create(
                "Weekend".to_string(),
                None,
                Some(root.id.to_string()),
                crate::types::CollectionVisibility::Private,
                None,
            )
            .await
            .unwrap();
        assert_eq!(weekend.parent_collection_id, Some(root.id));
        assert_eq!(weekend.sort_order, 1);
        let archive = service
            .create(
                "Archive".to_string(),
                None,
                None,
                crate::types::CollectionVisibility::Private,
                None,
            )
            .await
            .unwrap();
        assert_eq!(archive.parent_collection_id, None);
        assert_eq!(archive.sort_order, 1);

        let contents = service.get(root.id.to_string()).await.unwrap();
        assert_eq!(contents.collection.id, root.id);
        assert_eq!(contents.breadcrumbs.len(), 1);
        assert_eq!(contents.child_collections[0].id, child.id);
        assert_eq!(contents.child_collections[1].id, weekend.id);
        let nested_contents = service.get(grandchild.id.to_string()).await.unwrap();
        assert_eq!(
            nested_contents
                .breadcrumbs
                .iter()
                .map(|breadcrumb| breadcrumb.id)
                .collect::<Vec<_>>(),
            vec![root.id, child.id, grandchild.id]
        );

        service
            .reorder_children(
                Some(root.id.to_string()),
                vec![weekend.id.to_string(), child.id.to_string()],
            )
            .await
            .unwrap();
        let reordered_children = service.children(root.id.to_string()).await.unwrap();
        assert_eq!(reordered_children[0].id, weekend.id);
        assert_eq!(reordered_children[0].sort_order, 0);
        assert_eq!(reordered_children[1].id, child.id);
        assert_eq!(reordered_children[1].sort_order, 1);
        assert_eq!(
            service
                .reorder_children(Some(root.id.to_string()), vec![child.id.to_string()])
                .await,
            Err(CollectionApiError::InvalidOrder)
        );

        service
            .reorder_children(None, vec![archive.id.to_string(), root.id.to_string()])
            .await
            .unwrap();
        let reordered_roots = service.roots().await.unwrap();
        assert_eq!(reordered_roots[0].id, archive.id);
        assert_eq!(reordered_roots[0].sort_order, 0);
        assert_eq!(reordered_roots[1].id, root.id);
        assert_eq!(reordered_roots[1].sort_order, 1);

        assert_eq!(
            service
                .create(
                    "meals".to_string(),
                    None,
                    None,
                    crate::types::CollectionVisibility::Private,
                    None,
                )
                .await,
            Err(CollectionApiError::NameTaken)
        );
        let duplicate_parent = service
            .create(
                "Weeknight".to_string(),
                None,
                Some(archive.id.to_string()),
                crate::types::CollectionVisibility::Private,
                None,
            )
            .await
            .unwrap();

        let other_root = other_service
            .create(
                "Other root".to_string(),
                None,
                None,
                crate::types::CollectionVisibility::Private,
                None,
            )
            .await
            .unwrap();
        let other_roots = other_service.roots().await.unwrap();
        assert_eq!(other_roots.len(), 1);
        assert_eq!(other_roots[0].id, other_root.id);
        assert_eq!(
            service
                .create(
                    "Cross owner".to_string(),
                    None,
                    Some(other_root.id.to_string()),
                    crate::types::CollectionVisibility::Private,
                    None,
                )
                .await,
            Err(CollectionApiError::ParentNotFound)
        );
        assert_eq!(
            other_service.get(root.id.to_string()).await,
            Err(CollectionApiError::CollectionNotFound)
        );
        assert_eq!(
            other_service
                .update(root.id.to_string(), "Stolen".to_string(), None, None)
                .await,
            Err(CollectionApiError::CollectionNotFound)
        );

        assert_eq!(
            service
                .move_to(child.id.to_string(), Some(grandchild.id.to_string()))
                .await,
            Err(CollectionApiError::CollectionCycle)
        );
        assert_eq!(
            service
                .move_to(child.id.to_string(), Some(archive.id.to_string()))
                .await,
            Err(CollectionApiError::NameTaken)
        );
        let moved = service
            .move_to(child.id.to_string(), Some(duplicate_parent.id.to_string()))
            .await
            .unwrap();
        assert_eq!(moved.parent_collection_id, Some(duplicate_parent.id));
        let moved_to_root = service.move_to(child.id.to_string(), None).await.unwrap();
        assert_eq!(moved_to_root.parent_collection_id, None);
        let roots_after_move = service.roots().await.unwrap();
        assert_eq!(roots_after_move[2].id, child.id);
        assert_eq!(roots_after_move[2].sort_order, 2);

        let all_owned = service.all().await.unwrap();
        assert_eq!(all_owned.len(), 6);
        assert!(!all_owned
            .iter()
            .any(|collection| collection.id == other_root.id));

        assert_eq!(
            other_service.delete(root.id.to_string()).await,
            Err(CollectionApiError::CollectionNotFound)
        );
        assert!(service.get(root.id.to_string()).await.is_ok());
    }

    #[tokio::test]
    async fn service_applies_recipe_visibility_membership_and_reorder_rules() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        let author = test_utils::insert_test_user(&pool).await;
        let other_user = test_utils::insert_test_user(&pool).await;
        let service = CollectionService::new(&pool, owner.id);
        let other_service = CollectionService::new(&pool, other_user.id);
        let owner_reader = CollectionReadService::new(&pool, Some(owner.id));
        let other_reader = CollectionReadService::new(&pool, Some(other_user.id));
        let collection = service
            .create(
                "Saved".to_string(),
                None,
                None,
                crate::types::CollectionVisibility::Private,
                None,
            )
            .await
            .unwrap();
        let own_private =
            test_utils::create_test_published_recipe(&pool, owner.id, "Owned Published Recipe")
                .await;
        let draft = test_utils::create_test_new_draft(&pool, owner.id).await;
        let public = test_utils::insert_test_recipe(&pool, author.id, "public").await;
        let unlisted = test_utils::insert_test_recipe(&pool, author.id, "unlisted").await;
        let private = test_utils::insert_test_recipe(&pool, author.id, "private").await;

        service
            .add_recipe(collection.id.to_string(), own_private.id.to_string())
            .await
            .unwrap();
        service
            .add_recipe(collection.id.to_string(), public.id.to_string())
            .await
            .unwrap();
        service
            .add_recipe(collection.id.to_string(), unlisted.id.to_string())
            .await
            .unwrap();

        assert_eq!(
            service
                .add_recipe(collection.id.to_string(), own_private.id.to_string())
                .await,
            Err(CollectionApiError::RecipeAlreadyInCollection)
        );
        assert_eq!(
            service
                .add_recipe(collection.id.to_string(), private.id.to_string())
                .await,
            Err(CollectionApiError::RecipeNotAccessible)
        );
        assert_eq!(
            service
                .add_recipe(collection.id.to_string(), Uuid::new_v4().to_string())
                .await,
            Err(CollectionApiError::RecipeNotAccessible)
        );
        assert_eq!(
            service
                .add_recipe(collection.id.to_string(), draft.id.to_string())
                .await,
            Err(CollectionApiError::RecipeNotAccessible)
        );
        assert_eq!(
            other_service
                .add_recipe(collection.id.to_string(), public.id.to_string())
                .await,
            Err(CollectionApiError::CollectionNotFound)
        );
        assert_eq!(
            other_reader.recipes(collection.id.to_string(), 0, 20).await,
            Err(CollectionApiError::CollectionNotFound)
        );
        assert_eq!(
            owner_reader
                .recipes(collection.id.to_string(), -1, 20)
                .await,
            Err(CollectionApiError::InvalidPagination)
        );
        assert_eq!(
            owner_reader
                .recipes(collection.id.to_string(), 0, 101)
                .await,
            Err(CollectionApiError::InvalidPagination)
        );

        let listed = owner_reader
            .recipes(collection.id.to_string(), 0, 20)
            .await
            .unwrap();
        assert_eq!(listed.total_count, 3);
        assert_eq!(
            service
                .recipe_collections(own_private.id.to_string())
                .await
                .unwrap(),
            vec![collection.id]
        );

        service
            .reorder_recipes(
                collection.id.to_string(),
                vec![
                    unlisted.id.to_string(),
                    public.id.to_string(),
                    own_private.id.to_string(),
                ],
            )
            .await
            .unwrap();
        assert_eq!(
            owner_reader
                .recipes(collection.id.to_string(), 0, 20)
                .await
                .unwrap()
                .recipes[0]
                .id,
            unlisted.id
        );
        assert_eq!(
            service
                .reorder_recipes(collection.id.to_string(), vec![own_private.id.to_string()],)
                .await,
            Err(CollectionApiError::InvalidOrder)
        );

        sqlx::query("UPDATE recipes SET visibility = 'private' WHERE id = $1")
            .bind(public.id)
            .execute(&pool)
            .await
            .unwrap();
        service
            .reorder_recipes(
                collection.id.to_string(),
                vec![own_private.id.to_string(), unlisted.id.to_string()],
            )
            .await
            .unwrap();
        sqlx::query("UPDATE recipes SET visibility = 'public' WHERE id = $1")
            .bind(public.id)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            owner_reader
                .recipes(collection.id.to_string(), 0, 20)
                .await
                .unwrap()
                .recipes
                .iter()
                .map(|recipe| recipe.id)
                .collect::<Vec<_>>(),
            vec![own_private.id, unlisted.id, public.id]
        );

        service
            .remove_recipe(collection.id.to_string(), public.id.to_string())
            .await
            .unwrap();
        assert_eq!(
            owner_reader
                .recipes(collection.id.to_string(), 0, 20)
                .await
                .unwrap()
                .total_count,
            2
        );
    }

    #[tokio::test]
    async fn concurrent_service_root_creation_preserves_append_order() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        let mut tasks = Vec::new();
        for index in 0..8 {
            let pool = pool.clone();
            tasks.push(tokio::spawn(async move {
                CollectionService::new(&pool, owner.id)
                    .create(
                        format!("Root {index}"),
                        None,
                        None,
                        crate::types::CollectionVisibility::Private,
                        None,
                    )
                    .await
                    .unwrap()
            }));
        }
        for task in tasks {
            task.await.unwrap();
        }

        let roots = CollectionService::new(&pool, owner.id)
            .roots()
            .await
            .unwrap();
        assert_eq!(roots.len(), 8);
        assert_eq!(
            roots.iter().map(|root| root.sort_order).collect::<Vec<_>>(),
            (0..8).collect::<Vec<_>>()
        );
    }
}
