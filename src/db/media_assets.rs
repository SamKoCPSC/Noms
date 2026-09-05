use std::collections::{HashMap, HashSet};

use chrono::Utc;
use sqlx::{PgPool, Postgres, Row};
use uuid::Uuid;

use super::{DbError, MediaDiscardClaim, MediaDiscardDecision, MediaPurpose, NewMediaAsset};
use crate::types::{RecipeImage, RecipeStep};

#[derive(Debug, Clone, Copy)]
pub(crate) enum MediaAttachAuthority {
    UploaderOnly,
    Recipe(Uuid),
    Draft(Uuid),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum MediaHolder {
    Recipe(Uuid),
    Version(Uuid),
    Draft(Uuid),
    Collection(Uuid),
    Avatar(Uuid),
}

impl MediaHolder {
    fn table_and_column(self) -> (&'static str, &'static str, Uuid) {
        match self {
            Self::Recipe(id) => ("recipe_media_refs", "recipe_id", id),
            Self::Version(id) => ("recipe_version_media_refs", "recipe_version_id", id),
            Self::Draft(id) => ("recipe_draft_media_refs", "recipe_draft_id", id),
            Self::Collection(id) => ("collection_media_refs", "collection_id", id),
            Self::Avatar(id) => ("avatar_media_refs", "user_id", id),
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct LockedMediaAsset {
    id: Uuid,
    uploader_id: Option<Uuid>,
    purpose: MediaPurpose,
    full_url: String,
    thumbnail_url: String,
    lifecycle: String,
    expires_at: Option<chrono::DateTime<Utc>>,
}

pub fn recipe_media_asset_ids(images: &[RecipeImage], steps: &[RecipeStep]) -> Vec<Uuid> {
    fn collect_steps(steps: &[RecipeStep], ids: &mut Vec<Uuid>) {
        for step in steps {
            ids.extend(step.photos.iter().filter_map(|image| image.asset_id));
            collect_steps(&step.sub_steps, ids);
        }
    }

    let mut ids = images
        .iter()
        .filter_map(|image| image.asset_id)
        .collect::<Vec<_>>();
    collect_steps(steps, &mut ids);
    ids.sort_unstable();
    ids.dedup();
    ids
}

fn recipe_managed_images<'a>(
    images: &'a [RecipeImage],
    steps: &'a [RecipeStep],
) -> Vec<&'a RecipeImage> {
    fn collect_steps<'a>(steps: &'a [RecipeStep], images: &mut Vec<&'a RecipeImage>) {
        for step in steps {
            images.extend(step.photos.iter().filter(|image| image.asset_id.is_some()));
            collect_steps(&step.sub_steps, images);
        }
    }

    let mut managed = images
        .iter()
        .filter(|image| image.asset_id.is_some())
        .collect::<Vec<_>>();
    collect_steps(steps, &mut managed);
    managed
}

pub async fn register_media_asset(pool: &PgPool, asset: &NewMediaAsset) -> Result<(), DbError> {
    let mut transaction = pool.begin().await.map_err(DbError::Query)?;
    register_media_asset_in_transaction(&mut transaction, asset).await?;
    transaction.commit().await.map_err(DbError::Query)
}

pub(crate) async fn register_media_asset_in_transaction(
    transaction: &mut sqlx::Transaction<'_, Postgres>,
    asset: &NewMediaAsset,
) -> Result<(), DbError> {
    let lifecycle = if asset.purpose == MediaPurpose::Avatar {
        "retained"
    } else {
        "uncommitted"
    };
    sqlx::query(
        "INSERT INTO media_assets (id, uploader_id, purpose, full_url, thumbnail_url, \
             original_url, full_key, thumbnail_key, original_key, capability_digest, \
             lifecycle, expires_at, retained_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, \
             CASE WHEN $11 = 'retained' THEN NOW() ELSE NULL END)",
    )
    .bind(asset.id)
    .bind(asset.uploader_id)
    .bind(asset.purpose)
    .bind(&asset.full_url)
    .bind(&asset.thumbnail_url)
    .bind(&asset.original_url)
    .bind(&asset.full_key)
    .bind(&asset.thumbnail_key)
    .bind(&asset.original_key)
    .bind(&asset.capability_digest)
    .bind(lifecycle)
    .bind(asset.expires_at)
    .execute(&mut **transaction)
    .await
    .map_err(DbError::Query)?;
    Ok(())
}

async fn authorized_asset_ids(
    transaction: &mut sqlx::Transaction<'_, Postgres>,
    authority: MediaAttachAuthority,
) -> Result<HashSet<Uuid>, DbError> {
    let ids = match authority {
        MediaAttachAuthority::UploaderOnly => Vec::new(),
        MediaAttachAuthority::Draft(draft_id) => sqlx::query_scalar(
            "SELECT asset_id FROM recipe_draft_media_refs WHERE recipe_draft_id = $1",
        )
        .bind(draft_id)
        .fetch_all(&mut **transaction)
        .await
        .map_err(DbError::Query)?,
        MediaAttachAuthority::Recipe(recipe_id) => sqlx::query_scalar(
            "SELECT asset_id FROM recipe_media_refs WHERE recipe_id = $1 \
                 UNION SELECT ref.asset_id FROM recipe_version_media_refs ref \
                    JOIN recipe_versions version ON version.id = ref.recipe_version_id \
                    WHERE version.recipe_id = $1 \
                 UNION SELECT ref.asset_id FROM recipe_draft_media_refs ref \
                    JOIN recipe_drafts draft ON draft.id = ref.recipe_draft_id \
                    WHERE draft.recipe_id = $1",
        )
        .bind(recipe_id)
        .fetch_all(&mut **transaction)
        .await
        .map_err(DbError::Query)?,
    };
    Ok(ids.into_iter().collect())
}

async fn current_holder_asset_ids(
    transaction: &mut sqlx::Transaction<'_, Postgres>,
    holder: MediaHolder,
) -> Result<Vec<Uuid>, DbError> {
    let (table, column, holder_id) = holder.table_and_column();
    sqlx::query_scalar(&format!("SELECT asset_id FROM {table} WHERE {column} = $1"))
        .bind(holder_id)
        .fetch_all(&mut **transaction)
        .await
        .map_err(DbError::Query)
}

pub(crate) async fn sync_recipe_media_references(
    transaction: &mut sqlx::Transaction<'_, Postgres>,
    holder: MediaHolder,
    actor_id: Uuid,
    purpose: MediaPurpose,
    images: &[RecipeImage],
    steps: &[RecipeStep],
    authority: MediaAttachAuthority,
) -> Result<(), DbError> {
    let managed_images = recipe_managed_images(images, steps);
    let mut requested_ids = managed_images
        .iter()
        .filter_map(|image| image.asset_id)
        .collect::<Vec<_>>();
    requested_ids.sort_unstable();
    requested_ids.dedup();
    sync_media_references(
        transaction,
        holder,
        actor_id,
        purpose,
        &requested_ids,
        &managed_images,
        authority,
    )
    .await
}

async fn sync_media_references(
    transaction: &mut sqlx::Transaction<'_, Postgres>,
    holder: MediaHolder,
    actor_id: Uuid,
    purpose: MediaPurpose,
    requested_ids: &[Uuid],
    managed_images: &[&RecipeImage],
    authority: MediaAttachAuthority,
) -> Result<(), DbError> {
    let mut locked_ids = current_holder_asset_ids(transaction, holder).await?;
    locked_ids.extend_from_slice(requested_ids);
    locked_ids.sort_unstable();
    locked_ids.dedup();

    let locked = if locked_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as::<_, LockedMediaAsset>(
            "SELECT id, uploader_id, purpose, full_url, thumbnail_url, lifecycle, expires_at \
             FROM media_assets WHERE id = ANY($1) ORDER BY id FOR UPDATE",
        )
        .bind(&locked_ids)
        .fetch_all(&mut **transaction)
        .await
        .map_err(DbError::Query)?
    };
    let by_id = locked
        .iter()
        .map(|asset| (asset.id, asset))
        .collect::<HashMap<_, _>>();
    let inherited = authorized_asset_ids(transaction, authority).await?;
    for requested_id in requested_ids {
        let asset = by_id.get(requested_id).ok_or_else(|| {
            DbError::InvalidRecipeSnapshot("managed media asset was not found".to_string())
        })?;
        if asset.purpose != purpose
            || matches!(asset.lifecycle.as_str(), "delete_claimed" | "deleted")
            || (asset.lifecycle == "uncommitted"
                && asset.expires_at.is_some_and(|expiry| expiry <= Utc::now()))
            || (asset.uploader_id != Some(actor_id) && !inherited.contains(requested_id))
        {
            return Err(DbError::InvalidRecipeSnapshot(
                "managed media asset is not attachable to this holder".to_string(),
            ));
        }
    }
    for image in managed_images {
        let asset = by_id
            .get(&image.asset_id.expect("filtered managed image"))
            .ok_or_else(|| {
                DbError::InvalidRecipeSnapshot("managed media asset was not found".to_string())
            })?;
        if image.url != asset.full_url
            || image.thumb_url.as_deref() != Some(asset.thumbnail_url.as_str())
        {
            return Err(DbError::InvalidRecipeSnapshot(
                "managed media URLs do not match the asset identity".to_string(),
            ));
        }
    }

    let (table, column, holder_id) = holder.table_and_column();
    sqlx::query(&format!("DELETE FROM {table} WHERE {column} = $1"))
        .bind(holder_id)
        .execute(&mut **transaction)
        .await
        .map_err(DbError::Query)?;
    if !requested_ids.is_empty() {
        sqlx::query(&format!(
            "INSERT INTO {table} ({column}, asset_id) \
             SELECT $1, asset_id FROM UNNEST($2::UUID[]) AS asset_id"
        ))
        .bind(holder_id)
        .bind(requested_ids)
        .execute(&mut **transaction)
        .await
        .map_err(DbError::Query)?;
        sqlx::query(
            "UPDATE media_assets SET lifecycle = 'retained', \
                 retained_at = COALESCE(retained_at, NOW()) \
             WHERE id = ANY($1) AND lifecycle = 'uncommitted'",
        )
        .bind(requested_ids)
        .execute(&mut **transaction)
        .await
        .map_err(DbError::Query)?;
    }
    Ok(())
}

pub async fn replace_avatar_media_reference(
    pool: &PgPool,
    user_id: Uuid,
    asset_id: Uuid,
) -> Result<(), DbError> {
    let mut transaction = pool.begin().await.map_err(DbError::Query)?;
    replace_avatar_media_reference_in_transaction(&mut transaction, user_id, asset_id).await?;
    transaction.commit().await.map_err(DbError::Query)
}

pub(crate) async fn replace_avatar_media_reference_in_transaction(
    transaction: &mut sqlx::Transaction<'_, Postgres>,
    user_id: Uuid,
    asset_id: Uuid,
) -> Result<Option<Uuid>, DbError> {
    let previous = sqlx::query_scalar("SELECT asset_id FROM avatar_media_refs WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(DbError::Query)?;
    let mut lock_ids = previous
        .into_iter()
        .chain(std::iter::once(asset_id))
        .collect::<Vec<_>>();
    lock_ids.sort_unstable();
    lock_ids.dedup();
    let asset = sqlx::query_as::<_, LockedMediaAsset>(
        "SELECT id, uploader_id, purpose, full_url, thumbnail_url, lifecycle, expires_at \
         FROM media_assets WHERE id = ANY($1) ORDER BY id FOR UPDATE",
    )
    .bind(&lock_ids)
    .fetch_all(&mut **transaction)
    .await
    .map_err(DbError::Query)?
    .into_iter()
    .find(|candidate| candidate.id == asset_id)
    .ok_or_else(|| DbError::InvalidRecipeSnapshot("avatar asset was not found".to_string()))?;
    if asset.uploader_id != Some(user_id)
        || asset.purpose != MediaPurpose::Avatar
        || asset.lifecycle != "retained"
    {
        return Err(DbError::InvalidRecipeSnapshot(
            "avatar asset is not attachable".to_string(),
        ));
    }
    sqlx::query(
        "INSERT INTO avatar_media_refs (user_id, asset_id) VALUES ($1, $2) \
         ON CONFLICT (user_id) DO UPDATE SET asset_id = EXCLUDED.asset_id",
    )
    .bind(user_id)
    .bind(asset_id)
    .execute(&mut **transaction)
    .await
    .map_err(DbError::Query)?;
    Ok(previous)
}

pub(crate) async fn clear_avatar_media_reference_in_transaction(
    transaction: &mut sqlx::Transaction<'_, Postgres>,
    user_id: Uuid,
) -> Result<Option<Uuid>, DbError> {
    let previous =
        sqlx::query_scalar("SELECT asset_id FROM avatar_media_refs WHERE user_id = $1 FOR UPDATE")
            .bind(user_id)
            .fetch_optional(&mut **transaction)
            .await
            .map_err(DbError::Query)?;
    if let Some(asset_id) = previous {
        sqlx::query("SELECT id FROM media_assets WHERE id = $1 FOR UPDATE")
            .bind(asset_id)
            .fetch_one(&mut **transaction)
            .await
            .map_err(DbError::Query)?;
    }
    sqlx::query("DELETE FROM avatar_media_refs WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut **transaction)
        .await
        .map_err(DbError::Query)?;
    Ok(previous)
}

async fn asset_has_reference(
    transaction: &mut sqlx::Transaction<'_, Postgres>,
    asset_id: Uuid,
) -> Result<bool, DbError> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM recipe_media_refs WHERE asset_id = $1) \
          OR EXISTS(SELECT 1 FROM recipe_version_media_refs WHERE asset_id = $1) \
          OR EXISTS(SELECT 1 FROM recipe_draft_media_refs WHERE asset_id = $1) \
          OR EXISTS(SELECT 1 FROM collection_media_refs WHERE asset_id = $1) \
          OR EXISTS(SELECT 1 FROM avatar_media_refs WHERE asset_id = $1)",
    )
    .bind(asset_id)
    .fetch_one(&mut **transaction)
    .await
    .map_err(DbError::Query)
}

async fn claim_locked_asset(
    transaction: &mut sqlx::Transaction<'_, Postgres>,
    row: sqlx::postgres::PgRow,
    direct_discard: bool,
) -> Result<MediaDiscardDecision, DbError> {
    let id: Uuid = row.try_get("id").map_err(DbError::Query)?;
    let lifecycle: String = row.try_get("lifecycle").map_err(DbError::Query)?;
    let expires_at: Option<chrono::DateTime<Utc>> =
        row.try_get("expires_at").map_err(DbError::Query)?;
    if asset_has_reference(transaction, id).await? {
        sqlx::query(
            "UPDATE media_assets SET lifecycle = 'retained', cleanup_checked_at = NOW(), \
                 retained_at = COALESCE(retained_at, NOW()) WHERE id = $1",
        )
        .bind(id)
        .execute(&mut **transaction)
        .await
        .map_err(DbError::Query)?;
        return Ok(MediaDiscardDecision::Retained);
    }
    if lifecycle == "delete_claimed" && !direct_discard {
        return Ok(MediaDiscardDecision::Claimed(media_claim_from_row(row)?));
    }
    if matches!(lifecycle.as_str(), "delete_claimed" | "deleted") {
        return Ok(MediaDiscardDecision::Denied);
    }
    if direct_discard
        && (lifecycle != "uncommitted" || expires_at.is_none_or(|expiry| expiry <= Utc::now()))
    {
        return Ok(MediaDiscardDecision::Denied);
    }
    if !direct_discard
        && lifecycle == "uncommitted"
        && expires_at.is_some_and(|value| value > Utc::now())
    {
        return Ok(MediaDiscardDecision::Retained);
    }
    sqlx::query(
        "UPDATE media_assets SET lifecycle = 'delete_claimed', delete_claimed_at = NOW(), \
             cleanup_checked_at = NOW(), delete_failure = NULL WHERE id = $1",
    )
    .bind(id)
    .execute(&mut **transaction)
    .await
    .map_err(DbError::Query)?;
    Ok(MediaDiscardDecision::Claimed(media_claim_from_row(row)?))
}

fn media_claim_from_row(row: sqlx::postgres::PgRow) -> Result<MediaDiscardClaim, DbError> {
    Ok(MediaDiscardClaim {
        id: row.try_get("id").map_err(DbError::Query)?,
        full_key: row.try_get("full_key").map_err(DbError::Query)?,
        thumbnail_key: row.try_get("thumbnail_key").map_err(DbError::Query)?,
        original_key: row.try_get("original_key").map_err(DbError::Query)?,
    })
}

const CLAIM_COLUMNS: &str = "id, full_key, thumbnail_key, original_key, lifecycle, expires_at";

pub async fn claim_media_discard_by_digest(
    pool: &PgPool,
    uploader_id: Uuid,
    capability_digest: &[u8],
) -> Result<MediaDiscardDecision, DbError> {
    let mut transaction = pool.begin().await.map_err(DbError::Query)?;
    let row = sqlx::query(&format!(
        "SELECT {CLAIM_COLUMNS} FROM media_assets \
         WHERE capability_digest = $1 AND uploader_id = $2 FOR UPDATE"
    ))
    .bind(capability_digest)
    .bind(uploader_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(DbError::Query)?;
    let decision = match row {
        Some(row) => claim_locked_asset(&mut transaction, row, true).await?,
        None => MediaDiscardDecision::Denied,
    };
    transaction.commit().await.map_err(DbError::Query)?;
    Ok(decision)
}

pub async fn claim_media_cleanup(
    pool: &PgPool,
    asset_id: Uuid,
) -> Result<MediaDiscardDecision, DbError> {
    let mut transaction = pool.begin().await.map_err(DbError::Query)?;
    let row = sqlx::query(&format!(
        "SELECT {CLAIM_COLUMNS} FROM media_assets WHERE id = $1 FOR UPDATE"
    ))
    .bind(asset_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(DbError::Query)?;
    let decision = match row {
        Some(row) => claim_locked_asset(&mut transaction, row, false).await?,
        None => MediaDiscardDecision::Denied,
    };
    transaction.commit().await.map_err(DbError::Query)?;
    Ok(decision)
}

pub async fn list_media_asset_cleanup_candidates(pool: &PgPool) -> Result<Vec<Uuid>, DbError> {
    const BATCH_LIMIT: i64 = 500;
    sqlx::query_scalar(
        "WITH candidates AS ( \
             (SELECT id, 0 AS priority, cleanup_checked_at, created_at \
              FROM media_assets WHERE lifecycle = 'delete_claimed' \
              ORDER BY cleanup_checked_at NULLS FIRST, created_at, id LIMIT $1) \
             UNION ALL \
             (SELECT id, 1 AS priority, cleanup_checked_at, created_at \
              FROM media_assets WHERE lifecycle = 'uncommitted' AND expires_at <= NOW() \
              ORDER BY expires_at, created_at, id LIMIT $1) \
             UNION ALL \
             (SELECT id, 2 AS priority, cleanup_checked_at, created_at \
              FROM media_assets WHERE lifecycle = 'retained' \
              ORDER BY cleanup_checked_at NULLS FIRST, created_at, id LIMIT $1) \
         ) SELECT id FROM candidates \
         ORDER BY priority, cleanup_checked_at NULLS FIRST, created_at, id LIMIT $1",
    )
    .bind(BATCH_LIMIT)
    .fetch_all(pool)
    .await
    .map_err(DbError::Query)
}

pub async fn finish_media_asset_delete(
    pool: &PgPool,
    asset_id: Uuid,
    delete_failure: Option<&str>,
) -> Result<(), DbError> {
    let lifecycle = if delete_failure.is_some() {
        "delete_claimed"
    } else {
        "deleted"
    };
    sqlx::query(
        "UPDATE media_assets SET lifecycle = $2, \
             deleted_at = CASE WHEN $2 = 'deleted' THEN NOW() ELSE deleted_at END, \
             delete_failure = LEFT($3, 4096) WHERE id = $1 AND lifecycle = 'delete_claimed'",
    )
    .bind(asset_id)
    .bind(lifecycle)
    .bind(delete_failure)
    .execute(pool)
    .await
    .map_err(DbError::Query)?;
    Ok(())
}
