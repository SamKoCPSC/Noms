//! Owner-only recovery metadata in draft JSONB, outside recipe content.
use super::*;
use crate::types::{ImportImageStatus, ImportedRecipeImage};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JsonImportReview {
    pub source: Option<Uuid>,
    pub images: Vec<ImportedRecipeImage>,
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
