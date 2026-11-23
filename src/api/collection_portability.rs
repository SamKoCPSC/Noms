//! Collection portability boundaries; imported identities never grant database access.
use crate::collection_json::{
    CollectionImportNames, CollectionImportPreview, CollectionImportResult,
};

#[cfg(all(test, feature = "server"))]
#[path = "collection_portability_tests.rs"]
pub(crate) mod tests;
use dioxus::prelude::*;

#[server]
pub async fn export_collection_json(collection_id: String) -> Result<String, ServerFnError> {
    let user = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    crate::security::enforce_user_request_budget(
        user,
        crate::security::RequestClass::CollectionRead,
    )
    .map_err(|_| ServerFnError::new("Too many collection exports. Try again shortly."))?;
    let id = uuid::Uuid::parse_str(&collection_id)
        .map_err(|_| ServerFnError::new("Invalid collection ID"))?;
    crate::security::run_with_expensive_read_work_budget(crate::security::run_with_database_budget(
        crate::security::DATABASE_EXECUTION_BUDGET,
        crate::db::export_collection_json(&crate::db::get_pool(), user, id),
    ))
    .await
    .map_err(|_| ServerFnError::new("Exports are busy. Try again shortly."))?
    .map_err(|_| ServerFnError::new("Collection export timed out. Try a smaller collection."))?
    .map_err(ServerFnError::new)
}

#[server]
pub async fn inspect_collection_json(
    expected_account_id: Option<uuid::Uuid>,
    json: String,
) -> Result<CollectionImportPreview, ServerFnError> {
    let user = crate::auth::account_binding::require_form_account(expected_account_id).await?;
    crate::security::enforce_user_request_budget(
        user,
        crate::security::RequestClass::CollectionRead,
    )
    .map_err(|_| ServerFnError::new("Too many file checks. Try again shortly."))?;
    let parsed = crate::security::run_with_expensive_read_work_budget(async {
        crate::collection_json::parse(&json)
    })
    .await
    .map_err(|_| ServerFnError::new("File checks are busy. Try again shortly."))?
    .map_err(ServerFnError::new)?;
    let storage = crate::storage::get_storage_client();
    let mut collections = parsed
        .snapshot
        .collections
        .iter()
        .map(|node| crate::collection_json::CollectionImportNamePreview {
            portable_id: node.portable_id.clone(),
            name: node.name.clone(),
            already_exists: false,
        })
        .collect::<Vec<_>>();
    let mut recipes = parsed
        .recipes
        .iter()
        .map(
            |(file, editor)| crate::collection_json::CollectionImportNamePreview {
                portable_id: file.portable_id.clone(),
                name: editor.title.clone(),
                already_exists: false,
            },
        )
        .collect::<Vec<_>>();
    crate::security::run_with_database_budget(crate::security::DATABASE_EXECUTION_BUDGET, async {
        mark_existing_names(&crate::db::get_pool(), user, &mut collections, true).await?;
        mark_existing_names(&crate::db::get_pool(), user, &mut recipes, false).await
    })
    .await
    .map_err(|_| ServerFnError::new("Name checks timed out. Check the file again."))?
    .map_err(|_| ServerFnError::new("Could not check imported names. Check the file again."))?;
    let mut urls = vec![];
    for (_, mut editor) in parsed.recipes {
        urls.extend(super::recipe_json_media::external_urls(
            &mut editor,
            &storage,
        ));
    }
    urls.sort();
    urls.dedup();
    Ok(CollectionImportPreview {
        owner_id: user,
        name: parsed.snapshot.collections[0].name.clone(),
        collection_count: parsed.snapshot.collections.len(),
        recipe_count: parsed.snapshot.recipes.len(),
        external_image_urls: urls,
        collections,
        recipes,
    })
}

#[server]
pub async fn import_collection_json(
    expected_account_id: Option<uuid::Uuid>,
    json: String,
    available_external_urls: Vec<String>,
    names: Option<CollectionImportNames>,
) -> Result<CollectionImportResult, ServerFnError> {
    let user = crate::auth::account_binding::require_form_account(expected_account_id).await?;
    crate::security::enforce_user_request_budget(user, crate::security::RequestClass::Mutation)
        .map_err(|_| ServerFnError::new("Too many imports. Try again shortly."))?;
    crate::security::run_with_expensive_read_work_budget(import_for_user(
        &crate::db::get_pool(),
        &crate::storage::get_storage_client(),
        user,
        &json,
        &available_external_urls,
        &names.unwrap_or_default(),
    ))
    .await
    .map_err(|_| ServerFnError::new("Image checks are busy. Try again shortly."))?
}

#[cfg(feature = "server")]
pub(crate) async fn import_for_user(
    pool: &sqlx::PgPool,
    storage: &crate::storage::StorageClient,
    user: uuid::Uuid,
    json: &str,
    available: &[String],
    names: &CollectionImportNames,
) -> Result<CollectionImportResult, ServerFnError> {
    if available.len() > crate::collection_json::MAX_IMAGES
        || available
            .iter()
            .any(|u| u.len() > crate::security::MAX_IMAGE_METADATA_BYTES)
    {
        return Err(ServerFnError::new("Too many image checks"));
    }
    let mut parsed = crate::collection_json::parse(json).map_err(ServerFnError::new)?;
    parsed.apply_names(names).map_err(ServerFnError::new)?;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(12);
    let mut prepared = vec![];
    for (file, editor) in parsed.recipes {
        let source = crate::recipe_json::source_recipe_id(&file).map_err(ServerFnError::new)?;
        prepared.push(
            super::recipe_json_media::prepare_collection_recipe(
                pool, storage, user, source, editor, available, deadline,
            )
            .await?,
        );
    }
    let mut warnings = vec![];
    let mut covers = std::collections::HashMap::new();
    let ids = parsed
        .snapshot
        .collections
        .iter()
        .filter_map(|c| c.image.as_ref().and_then(|image| image.asset_id))
        .collect::<Vec<_>>();
    let allowed: Vec<(uuid::Uuid, String)> = if ids.is_empty()
        || tokio::time::Instant::now() >= deadline
    {
        vec![]
    } else {
        crate::security::run_with_database_budget(crate::security::DATABASE_EXECUTION_BUDGET.min(deadline.saturating_duration_since(tokio::time::Instant::now())),
        sqlx::query_as("SELECT id, full_url FROM media_assets WHERE uploader_id=$1 AND id=ANY($2) AND purpose='collection_cover' AND lifecycle='retained'")
            .bind(user).bind(&ids).fetch_all(pool)
    ).await.map_err(|_| ServerFnError::new("Cover permission check timed out"))?
        .map_err(|_| ServerFnError::new("Could not check cover permissions"))?
    };
    for node in &parsed.snapshot.collections {
        if let Some(image) = &node.image {
            let owned = allowed
                .iter()
                .any(|(id, url)| Some(*id) == image.asset_id && *url == image.url)
                && storage
                    .validate_owned_media_pair(
                        &image.url,
                        image.thumb_url.as_deref(),
                        user,
                        crate::storage::MediaObjectKind::CollectionFull,
                    )
                    .is_ok();
            let exists = if owned {
                if let Ok(object) = storage.parse_media_url(&image.url) {
                    tokio::time::timeout_at(
                        deadline
                            .min(tokio::time::Instant::now() + std::time::Duration::from_secs(3)),
                        storage.export_image_exists(object.key()),
                    )
                    .await
                    .is_ok_and(|result| matches!(result, Ok(true)))
                } else {
                    false
                }
            } else {
                false
            };
            if exists {
                covers.insert(node.portable_id.clone(), image.clone());
            } else {
                warnings.push(format!("The cover for ‘{}’ could not be reused. Upload a replacement in Edit collection.", node.name));
            }
        }
    }
    crate::security::run_with_database_budget(crate::security::DATABASE_EXECUTION_BUDGET,
        crate::db::create_collection_json_import(pool, user, parsed.snapshot.collections, prepared, covers, warnings)
    ).await.map_err(|_| ServerFnError::new("Import could not be confirmed in time. Check your collections before retrying."))?
        .map_err(|error| match error {
            crate::db::DbError::CollectionNameTaken => ServerFnError::new("A collection in this import uses a name you already have, or repeats a name within the import. Change its name in the preview and try again. Your original file is unchanged."),
            crate::db::DbError::RecipeNameTaken => ServerFnError::new("A recipe in this collection uses a title you already have, or repeats a title within the import. Change its title in the preview and try again. Your original file is unchanged."),
            crate::db::DbError::ResourceLimitExceeded(_) | crate::db::DbError::CollectionLimitExceeded(_) => ServerFnError::new("Import exceeds your collection, recipe-version or image-review draft capacity. Free some capacity or import a smaller collection."),
            _ => ServerFnError::new("The collection import could not be completed. Check your collections before retrying."),
        })
}

#[cfg(feature = "server")]
async fn mark_existing_names(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    entries: &mut [crate::collection_json::CollectionImportNamePreview],
    collections: bool,
) -> Result<(), sqlx::Error> {
    if entries.is_empty() {
        return Ok(());
    }
    let ids = entries
        .iter()
        .map(|entry| entry.portable_id.clone())
        .collect::<Vec<_>>();
    let names = entries
        .iter()
        .map(|entry| entry.name.clone())
        .collect::<Vec<_>>();
    // Match the database's exact uniqueness expressions, including whitespace
    // normalization. Only names belonging to the authenticated actor are read.
    let query = if collections {
        "SELECT incoming.portable_id FROM unnest($2::text[], $3::text[]) AS incoming(portable_id, name) \
         WHERE EXISTS (SELECT 1 FROM collections WHERE owner_id=$1 AND \
         lower(btrim(regexp_replace(name, '[[:space:]]+', ' ', 'g'))) = \
         lower(btrim(regexp_replace(incoming.name, '[[:space:]]+', ' ', 'g'))))"
    } else {
        "SELECT incoming.portable_id FROM unnest($2::text[], $3::text[]) AS incoming(portable_id, name) \
         WHERE EXISTS (SELECT 1 FROM recipes WHERE user_id=$1 AND \
         lower(btrim(regexp_replace(title, '[[:space:]]+', ' ', 'g'))) = \
         lower(btrim(regexp_replace(incoming.name, '[[:space:]]+', ' ', 'g'))))"
    };
    let taken = sqlx::query_scalar::<_, String>(query)
        .bind(user)
        .bind(ids)
        .bind(names)
        .fetch_all(pool)
        .await?;
    for entry in entries {
        entry.already_exists = taken.contains(&entry.portable_id);
    }
    Ok(())
}
