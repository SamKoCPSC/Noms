//! Owner-only recovery metadata in draft JSONB, outside recipe content.
use super::*;
use crate::types::{ImportImageStatus, ImportedRecipeImage};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JsonImportReview {
    pub source: Option<Uuid>,
    pub images: Vec<ImportedRecipeImage>,
}

pub struct PreparedJsonRecipe {
    pub portable_id: String,
    pub payload: RecipeDraftPayload,
    pub review: JsonImportReview,
    pub checked: Vec<ImportedRecipeImage>,
}

/// Recreate a private collection tree and private saved copies in one commit.
pub async fn create_collection_json_import(
    pool: &PgPool,
    user: Uuid,
    nodes: Vec<crate::collection_json::PortableCollection>,
    prepared: Vec<PreparedJsonRecipe>,
    covers: HashMap<String, crate::types::RecipeImage>,
    mut warnings: Vec<String>,
) -> Result<crate::collection_json::CollectionImportResult, DbError> {
    let invalid = || DbError::InvalidRecipeSnapshot("Invalid collection import.".into());
    if nodes.is_empty()
        || nodes.len() > crate::collection_json::MAX_COLLECTIONS
        || prepared.len() > crate::collection_json::MAX_RECIPES
    {
        return Err(invalid());
    }
    let mut tx = pool.begin().await.map_err(DbError::Query)?;
    lock_resource_owner(&mut tx, user).await?;
    let (count, roots): (i64, i64) = sqlx::query_as("SELECT COUNT(*), COUNT(*) FILTER (WHERE parent_collection_id IS NULL) FROM collections WHERE owner_id=$1")
        .bind(user).fetch_one(&mut *tx).await.map_err(DbError::Query)?;
    if count as usize + nodes.len() > crate::security::MAX_COLLECTIONS_PER_USER
        || roots as usize >= crate::security::MAX_COLLECTION_CHILDREN
    {
        return Err(DbError::CollectionLimitExceeded(
            "collection import capacity",
        ));
    }
    let existing_names: Vec<String> = sqlx::query_scalar(
        "SELECT lower(name) FROM collections WHERE owner_id=$1 AND parent_collection_id IS NULL",
    )
    .bind(user)
    .fetch_all(&mut *tx)
    .await
    .map_err(DbError::Query)?;
    let original_name = &nodes[0].name;
    let mut root_name = original_name.clone();
    for copy in 1..=crate::security::MAX_COLLECTION_CHILDREN + 1 {
        if !existing_names.contains(&root_name.to_lowercase()) {
            break;
        }
        let suffix = if copy == 1 {
            " (imported)".into()
        } else {
            format!(" (imported {copy})")
        };
        root_name = original_name
            .chars()
            .take(crate::security::MAX_COLLECTION_NAME_CHARS - suffix.chars().count())
            .collect::<String>()
            + &suffix;
    }
    if root_name != *original_name {
        warnings.push(format!(
            "The existing collection was kept. The imported copy is named ‘{root_name}’."
        ));
    }
    let mut recipe_ids = HashMap::new();
    let mut image_reviews = vec![];
    for mut item in prepared {
        let source = apply_checked(
            &mut tx,
            user,
            None,
            &mut item.payload,
            &mut item.review,
            &item.checked,
        )
        .await?;
        let snapshot = normalized_snapshot(&item.payload.content);
        validate_snapshot_for_publish(&snapshot, "private")?;
        let recipe = insert_recipe_from_snapshot(
            &mut tx,
            user,
            &snapshot,
            "private",
            super::super::MediaAttachAuthority::JsonImport {
                source,
                draft: Uuid::nil(),
            },
        )
        .await?;
        insert_recipe_version(&mut tx, recipe.id, user, 1, &snapshot, None).await?;
        super::super::insert_recipe_tags(&mut tx, recipe.id, &snapshot.tags).await?;
        if !item.review.images.is_empty() {
            item.payload.initial_visibility = None;
            let draft = insert_recipe_draft_with_source(
                &mut tx,
                user,
                Some(recipe.id),
                Some(1),
                &item.payload,
                None,
                None,
                None,
                None,
            )
            .await?;
            write_review(&mut tx, draft.id, user, &item.payload, &item.review).await?;
            image_reviews.push(crate::collection_json::CollectionImageReview {
                recipe_id: recipe.id,
                draft_id: draft.id,
                title: recipe.title.clone(),
                image_count: item.review.images.len(),
            });
        }
        if recipe_ids.insert(item.portable_id, recipe.id).is_some() {
            return Err(invalid());
        }
    }
    let root_order: i32 = sqlx::query_scalar("SELECT COALESCE(MAX(sort_order),-1)+1 FROM collections WHERE owner_id=$1 AND parent_collection_id IS NULL")
        .bind(user).fetch_one(&mut *tx).await.map_err(DbError::Query)?;
    let mut collection_ids = HashMap::new();
    let mut orders = HashMap::<Uuid, i32>::new();
    let mut root = None;
    for (index, node) in nodes.iter().enumerate() {
        let parent = node
            .parent_collection
            .as_ref()
            .map(|p| collection_ids.get(p).copied().ok_or_else(invalid))
            .transpose()?;
        if (index == 0) != parent.is_none() {
            return Err(invalid());
        }
        let order = if let Some(parent) = parent {
            let order = orders.entry(parent).or_default();
            let current = *order;
            *order += 1;
            current
        } else {
            root_order
        };
        let name = if index == 0 {
            root_name.as_str()
        } else {
            node.name.as_str()
        };
        let id = super::super::insert_collection_import_node(
            &mut tx,
            user,
            parent,
            name,
            node,
            order,
            covers.get(&node.portable_id),
        )
        .await?;
        if index == 0 {
            root = Some(id);
        }
        if collection_ids
            .insert(node.portable_id.clone(), id)
            .is_some()
        {
            return Err(invalid());
        }
        let members = node
            .recipes
            .iter()
            .map(|r| recipe_ids.get(r).copied().ok_or_else(invalid))
            .collect::<Result<Vec<_>, _>>()?;
        sqlx::query("INSERT INTO collection_recipes (collection_id, recipe_id, sort_order) SELECT $1, recipe, (ordinality-1)::integer FROM unnest($2::uuid[]) WITH ORDINALITY AS members(recipe, ordinality)")
            .bind(id).bind(&members).execute(&mut *tx).await.map_err(DbError::Query)?;
    }
    let result = crate::collection_json::CollectionImportResult {
        owner_id: user,
        collection_id: root.ok_or_else(invalid)?,
        collection_count: collection_ids.len(),
        recipe_count: recipe_ids.len(),
        warnings,
        image_reviews,
    };
    tx.commit().await.map_err(DbError::Query)?;
    Ok(result)
}
pub fn json_import_review(draft: &RecipeDraft) -> Result<JsonImportReview, DbError> {
    draft
        .snapshot
        .get("_import_image_review")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(DbError::SerdeJson)
        .map(Option::unwrap_or_default)
}
/// Check permission before any storage request. Imported IDs never grant authority.
pub async fn json_import_asset_access(
    pool: &PgPool,
    user: Uuid,
    source: Option<Uuid>,
    ids: &[Uuid],
    draft: Option<Uuid>,
) -> Result<HashMap<String, Uuid>, DbError> {
    sqlx::query_as::<_, (String, Uuid)>(
        "SELECT asset.full_url, asset.id FROM media_assets asset \
         WHERE asset.id = ANY($3) AND asset.purpose = 'recipe' AND asset.lifecycle = 'retained' \
         AND (asset.uploader_id = $1 OR EXISTS (SELECT 1 FROM recipe_media_refs ref JOIN recipes r ON r.id = ref.recipe_id \
         WHERE ref.asset_id = asset.id AND r.id = $2 AND (r.user_id = $1 OR r.visibility IN ('public','unlisted'))) OR EXISTS (SELECT 1 FROM recipe_draft_media_refs ref JOIN recipe_drafts d ON d.id = ref.recipe_draft_id WHERE ref.asset_id = asset.id AND d.id = $4 AND d.user_id = $1))"
    ).bind(user).bind(source).bind(ids).bind(draft).fetch_all(pool).await.map_err(DbError::Query).map(|rows| rows.into_iter().collect())
}
async fn attach_checked(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    user: Uuid,
    draft: Uuid,
    payload: &mut RecipeDraftPayload,
    review: &mut JsonImportReview,
    checked: &[ImportedRecipeImage],
) -> Result<(), DbError> {
    let source = apply_checked(tx, user, Some(draft), payload, review, checked).await?;
    super::super::sync_recipe_media_references(
        tx,
        super::super::MediaHolder::Draft(draft),
        user,
        super::super::MediaPurpose::Recipe,
        &payload.images,
        &payload.instructions,
        super::super::MediaAttachAuthority::JsonImport { source, draft },
    )
    .await
}

async fn apply_checked(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    user: Uuid,
    draft: Option<Uuid>,
    payload: &mut RecipeDraftPayload,
    review: &mut JsonImportReview,
    checked: &[ImportedRecipeImage],
) -> Result<Option<Uuid>, DbError> {
    let source = if let Some(source) = review.source {
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM recipes WHERE id = $1 AND (user_id = $2 OR visibility IN ('public','unlisted')) FOR SHARE")
            .bind(source).bind(user).fetch_optional(&mut **tx).await.map_err(DbError::Query)?
    } else {
        None
    };
    // Match cleanup's lock order, including existing draft references.
    let mut ids = checked
        .iter()
        .filter_map(|i| i.image.asset_id)
        .collect::<Vec<_>>();
    ids.extend(super::super::recipe_media_asset_ids(
        &payload.images,
        &payload.instructions,
    ));
    let assets = sqlx::query_as::<_, (Uuid, Option<Uuid>, String, String, String)>(
        "SELECT id, uploader_id, full_url, thumbnail_url, lifecycle FROM media_assets WHERE id = ANY($1) AND purpose = 'recipe' ORDER BY id FOR UPDATE"
    ).bind(&ids).fetch_all(&mut **tx).await.map_err(DbError::Query)?;
    let mut inherited: Vec<Uuid> = if let Some(source) = source {
        sqlx::query_scalar("SELECT asset_id FROM recipe_media_refs WHERE recipe_id = $1")
            .bind(source)
            .fetch_all(&mut **tx)
            .await
            .map_err(DbError::Query)?
    } else {
        vec![]
    };
    let held: Vec<Uuid> = sqlx::query_scalar(
        "SELECT asset_id FROM recipe_draft_media_refs WHERE recipe_draft_id = $1",
    )
    .bind(draft)
    .fetch_all(&mut **tx)
    .await
    .map_err(DbError::Query)?;
    inherited.extend(held);
    for candidate in checked {
        let mut candidate = candidate.clone();
        if let Some(id) = candidate.image.asset_id {
            let asset = assets.iter().find(|a| {
                a.0 == id
                    && a.4 == "retained"
                    && (a.1 == Some(user) || inherited.contains(&id))
                    && a.2 == candidate.image.url
            });
            let Some(asset) = asset else {
                candidate.status = ImportImageStatus::NotReusable;
                review.images.push(candidate);
                continue;
            };
            candidate.image.thumb_url = Some(asset.3.clone());
        }
        let destination = if candidate.step_path.is_empty() {
            Some(&mut payload.content.images)
        } else {
            let mut steps = &mut payload.content.instructions;
            let mut target = None;
            for (index, position) in candidate.step_path.iter().enumerate() {
                let Some(step) = steps.get_mut(*position) else {
                    break;
                };
                if index + 1 == candidate.step_path.len() {
                    if Some(step.text.as_str()) == candidate.step_text.as_deref() {
                        target = Some(&mut step.photos);
                    }
                    break;
                }
                steps = &mut step.sub_steps;
            }
            target
        };
        if let Some(images) = destination {
            images.insert(candidate.position.min(images.len()), candidate.image);
        } else {
            candidate.status = ImportImageStatus::PlacementChanged;
            review.images.push(candidate);
        }
    }
    validate_draft_payload(payload)?;
    Ok(source)
}
async fn write_review(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    draft: Uuid,
    user: Uuid,
    payload: &RecipeDraftPayload,
    review: &JsonImportReview,
) -> Result<RecipeDraft, DbError> {
    let mut value = serde_json::to_value(payload).map_err(DbError::SerdeJson)?;
    value["_import_image_review"] = serde_json::to_value(review).map_err(DbError::SerdeJson)?;
    sqlx::query_as::<_, RecipeDraft>(&format!("UPDATE recipe_drafts SET snapshot = $3, revision = revision + 1, updated_at = NOW() WHERE id = $1 AND user_id = $2 RETURNING {DRAFT_COLUMNS}"))
        .bind(draft).bind(user).bind(value).fetch_one(&mut **tx).await.map_err(DbError::Query)
}
pub async fn create_json_import_draft(
    pool: &PgPool,
    user: Uuid,
    mut payload: RecipeDraftPayload,
    mut review: JsonImportReview,
    checked: Vec<ImportedRecipeImage>,
) -> Result<RecipeDraft, DbError> {
    let mut tx = pool.begin().await.map_err(DbError::Query)?;
    let draft = insert_recipe_draft_with_source(
        &mut tx, user, None, None, &payload, None, None, None, None,
    )
    .await?;
    attach_checked(&mut tx, user, draft.id, &mut payload, &mut review, &checked).await?;
    let draft = write_review(&mut tx, draft.id, user, &payload, &review).await?;
    tx.commit().await.map_err(DbError::Query)?;
    Ok(draft)
}
pub async fn resolve_json_import_image(
    pool: &PgPool,
    user: Uuid,
    id: Uuid,
    revision: i64,
    image: Uuid,
    checked: Option<ImportedRecipeImage>,
    status: Option<ImportImageStatus>,
) -> Result<RecipeDraft, DbError> {
    let mut tx = pool.begin().await.map_err(DbError::Query)?;
    lock_resource_owner(&mut tx, user).await?;
    let draft = locked_draft(&mut tx, id, user).await?;
    if draft.revision != revision {
        return Err(DbError::DraftConflict {
            draft_id: id,
            expected: revision,
            actual: draft.revision,
        });
    }
    let mut payload = draft.decoded_snapshot()?;
    let mut review = json_import_review(&draft)?;
    let Some(index) = review.images.iter().position(|i| i.id == image) else {
        return Err(DbError::InvalidRecipeSnapshot(
            "Imported image not found".into(),
        ));
    };
    let mut original = review.images.remove(index);
    if let Some(status) = status {
        original.status = status;
        review.images.insert(index, original);
    }
    if let Some(checked) = checked {
        attach_checked(&mut tx, user, id, &mut payload, &mut review, &[checked]).await?;
    }
    let draft = write_review(&mut tx, id, user, &payload, &review).await?;
    tx.commit().await.map_err(DbError::Query)?;
    Ok(draft)
}
