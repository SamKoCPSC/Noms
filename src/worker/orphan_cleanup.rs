//! Orphan image cleanup worker.
//!
//! Periodically reconciles a bounded batch of tracked upload families against
//! exact database references. Unknown legacy objects are never enumerated or
//! treated as deletion authority.

use std::collections::HashSet;
use std::time::Duration;

use sqlx::PgPool;
use tracing::{debug, error, info, warn};

use crate::db;
use crate::storage::StorageClient;

/// Configuration for the orphan cleanup worker.
pub struct OrphanCleanupConfig {
    /// How often to run the cleanup cycle.
    pub interval: Duration,
}

impl OrphanCleanupConfig {
    const DEFAULT_INTERVAL_SECS: u64 = 3600;

    /// Read config from environment.
    ///
    /// `NOMS_ORPHAN_CLEANUP_INTERVAL_SECS` — interval in seconds (default: 3600 = 1 hour).
    pub fn from_env() -> Self {
        let value = std::env::var("NOMS_ORPHAN_CLEANUP_INTERVAL_SECS").ok();
        Self::from_interval_value(value.as_deref())
    }

    fn from_interval_value(value: Option<&str>) -> Self {
        let interval_secs = value
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(Self::DEFAULT_INTERVAL_SECS);
        Self {
            interval: Duration::from_secs(interval_secs),
        }
    }
}

/// Spawn the orphan cleanup background task.
///
/// The task runs on a periodic interval and retries only database-tracked,
/// lock-claimed object-family cleanup.
/// Dropped automatically on server shutdown.
pub fn spawn_orphan_cleanup(pool: PgPool, storage: StorageClient, config: OrphanCleanupConfig) {
    info!(
        interval_secs = config.interval.as_secs(),
        "Spawning orphan cleanup worker"
    );
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(config.interval);
        // Skip the first immediate tick
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Err(e) = run_cleanup(&pool, &storage).await {
                error!(error = %e, "Orphan cleanup cycle failed");
            }
        }
    });
}

/// Run a single orphan cleanup cycle.
pub async fn run_cleanup(
    pool: &PgPool,
    storage: &StorageClient,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Account deletion tombstones survive their owning user and media-family
    // rows, so process their bounded batch before ordinary tracked families.
    let account_objects_deleted = run_account_media_cleanup(pool, storage).await?;

    // Candidate selection and each exact reference check are independently
    // bounded. Never list the bucket or materialize the complete document
    // reference corpus merely to process this batch.
    let candidates = db::list_media_cleanup_candidates(pool).await?;
    let candidates_examined = candidates.len();
    let mut deleted_count = 0u64;
    for (owner_id, full_url) in candidates {
        let db::MediaDiscardDecision::Claimed(claim) =
            db::claim_retained_media_cleanup(pool, owner_id, &full_url).await?
        else {
            continue;
        };
        let full = storage.delete_object(&claim.full_key).await;
        let thumbnail = storage.delete_object(&claim.thumbnail_key).await;
        let failure = match (&full, &thumbnail) {
            (Ok(()), Ok(())) => None,
            _ => Some(format!("full={full:?}; thumbnail={thumbnail:?}")),
        };
        db::finish_media_discard(pool, claim.id, failure.as_deref()).await?;
        if let Some(error) = failure {
            warn!(claim_id = %claim.id, %error, "Failed to delete tracked orphan family");
        } else {
            deleted_count += 2;
            debug!(claim_id = %claim.id, "Deleted tracked orphan family");
        }
    }

    info!(
        candidates_examined,
        account_objects_deleted,
        orphans_deleted = deleted_count,
        "Orphan cleanup cycle complete"
    );
    Ok(())
}

fn account_media_cleanup_keys(claim: &db::AccountMediaCleanupClaim) -> Result<Vec<String>, String> {
    let mut keys = HashSet::new();
    match claim.purpose {
        db::AccountMediaPurpose::Recipe | db::AccountMediaPurpose::CollectionCover => {
            let full_key = claim
                .full_key
                .as_deref()
                .ok_or_else(|| "tracked account media is missing its full key".to_string())?;
            let thumbnail_key = claim
                .thumbnail_key
                .as_deref()
                .ok_or_else(|| "tracked account media is missing its thumbnail key".to_string())?;
            keys.insert(full_key.to_string());
            keys.insert(thumbnail_key.to_string());
        }
        db::AccountMediaPurpose::Avatar => {
            if let Some(full_key) = claim.full_key.as_deref() {
                keys.insert(full_key.to_string());
            }
            if let Some(thumbnail_key) = claim.thumbnail_key.as_deref() {
                keys.insert(thumbnail_key.to_string());
            }
            if let Some(original_key) = claim.original_key.as_deref() {
                keys.insert(original_key.to_string());
            }
            if keys.is_empty() {
                return Err("account avatar cleanup has no validated object keys".to_string());
            }
        }
    }
    let mut keys = keys.into_iter().collect::<Vec<_>>();
    keys.sort();
    Ok(keys)
}

pub(crate) async fn run_account_media_cleanup(
    pool: &PgPool,
    storage: &StorageClient,
) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    let candidates = db::list_account_media_cleanup_candidates(pool, 500).await?;
    let mut deleted_count = 0u64;
    for tombstone_id in candidates {
        let db::AccountMediaCleanupDecision::Claimed(claim) =
            db::claim_account_media_cleanup(pool, tombstone_id).await?
        else {
            continue;
        };
        let keys = match account_media_cleanup_keys(&claim) {
            Ok(keys) => keys,
            Err(error) => {
                db::finish_account_media_cleanup(pool, claim.id, Some(&error)).await?;
                warn!(claim_id = %claim.id, %error, "Invalid account-media cleanup claim");
                continue;
            }
        };
        let mut failures = Vec::new();
        for key in &keys {
            if let Err(error) = storage.delete_object(key).await {
                failures.push(format!("{key}: {error}"));
            }
        }
        let failure = if failures.is_empty() {
            None
        } else {
            Some(failures.join("; ").chars().take(4096).collect::<String>())
        };
        db::finish_account_media_cleanup(pool, claim.id, failure.as_deref()).await?;
        if let Some(error) = failure {
            warn!(claim_id = %claim.id, %error, "Failed to delete account media; retry queued");
        } else {
            deleted_count += keys.len() as u64;
            debug!(claim_id = %claim.id, objects = keys.len(), "Deleted account media tombstone");
        }
    }
    Ok(deleted_count)
}

#[cfg(test)]
pub(crate) fn collect_retained_media_keys(
    storage: &StorageClient,
    references: &[db::OwnedImageReference],
) -> HashSet<String> {
    let mut keys = HashSet::new();
    for reference in references {
        let Ok(object) = storage.parse_media_url(&reference.image_url) else {
            continue;
        };
        match (reference.media_kind, object.kind()) {
            (db::OwnedMediaKind::Recipe, crate::storage::MediaObjectKind::RecipeFull)
                if object.owner_id() == reference.owner_id || reference.allows_inherited_owner =>
            {
                keys.insert(object.key().to_string());
                if let Some(thumbnail_key) = object.paired_thumbnail_key() {
                    keys.insert(thumbnail_key);
                }
            }
            (db::OwnedMediaKind::Recipe, crate::storage::MediaObjectKind::RecipeThumbnail)
                if object.owner_id() == reference.owner_id || reference.allows_inherited_owner =>
            {
                keys.insert(object.key().to_string());
            }
            (
                db::OwnedMediaKind::CollectionCover,
                crate::storage::MediaObjectKind::CollectionFull,
            ) if object.owner_id() == reference.owner_id => {
                keys.insert(object.key().to_string());
                if let Some(thumbnail_key) = object.paired_thumbnail_key() {
                    keys.insert(thumbnail_key);
                }
            }
            (
                db::OwnedMediaKind::CollectionCover,
                crate::storage::MediaObjectKind::CollectionThumbnail,
            ) if object.owner_id() == reference.owner_id => {
                keys.insert(object.key().to_string());
            }
            _ => {}
        }
    }
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recipe_reference(owner_id: uuid::Uuid, image_url: String) -> db::OwnedImageReference {
        db::OwnedImageReference {
            owner_id,
            image_url,
            allows_inherited_owner: false,
            media_kind: db::OwnedMediaKind::Recipe,
        }
    }

    fn collection_reference(owner_id: uuid::Uuid, image_url: String) -> db::OwnedImageReference {
        db::OwnedImageReference {
            owner_id,
            image_url,
            allows_inherited_owner: false,
            media_kind: db::OwnedMediaKind::CollectionCover,
        }
    }

    #[test]
    fn test_orphan_cleanup_config_default() {
        let config = OrphanCleanupConfig::from_interval_value(None);
        assert_eq!(config.interval, Duration::from_secs(3600));
    }

    #[test]
    fn test_orphan_cleanup_config_from_env() {
        let config = OrphanCleanupConfig::from_interval_value(Some("42"));
        assert_eq!(config.interval, Duration::from_secs(42));
    }

    #[test]
    fn test_orphan_cleanup_config_invalid_env() {
        let config = OrphanCleanupConfig::from_interval_value(Some("not-a-number"));
        assert_eq!(config.interval, Duration::from_secs(3600));
    }

    #[test]
    fn test_thumbnail_keys_derived_from_full_keys() {
        let first_owner = uuid::Uuid::new_v4();
        let second_owner = uuid::Uuid::new_v4();
        let first_object = uuid::Uuid::new_v4();
        let second_object = uuid::Uuid::new_v4();
        let storage = crate::storage::StorageClient::for_test();
        let references = vec![
            recipe_reference(
                first_owner,
                storage.object_url(&format!("recipes/{first_owner}/{first_object}_full.webp")),
            ),
            recipe_reference(
                second_owner,
                storage.object_url(&format!("recipes/{second_owner}/{second_object}_full.webp")),
            ),
        ];

        let referenced_keys = collect_retained_media_keys(&storage, &references);

        // Full keys are present
        assert!(
            referenced_keys.contains(&format!("recipes/{first_owner}/{first_object}_full.webp"))
        );
        assert!(
            referenced_keys.contains(&format!("recipes/{second_owner}/{second_object}_full.webp"))
        );

        // Derived thumbnail keys are also present
        assert!(
            referenced_keys.contains(&format!("recipes/{first_owner}/{first_object}_thumb.webp"))
        );
        assert!(referenced_keys.contains(&format!(
            "recipes/{second_owner}/{second_object}_thumb.webp"
        )));
        assert_eq!(referenced_keys.len(), 4);
    }

    #[test]
    fn noncanonical_recipe_keys_are_not_retained() {
        let owner_id = uuid::Uuid::new_v4();
        let storage = crate::storage::StorageClient::for_test();
        let references = vec![recipe_reference(
            owner_id,
            storage.object_url(&format!("recipes/{owner_id}/legacy_image.webp")),
        )];

        assert!(collect_retained_media_keys(&storage, &references).is_empty());
    }

    #[test]
    fn foreign_or_malformed_recipe_references_do_not_retain_objects() {
        let owner_id = uuid::Uuid::new_v4();
        let victim_id = uuid::Uuid::new_v4();
        let owned_object = uuid::Uuid::new_v4();
        let victim_object = uuid::Uuid::new_v4();
        let storage = crate::storage::StorageClient::for_test();
        let references = vec![
            recipe_reference(
                owner_id,
                storage.object_url(&format!("recipes/{owner_id}/{owned_object}_full.webp")),
            ),
            recipe_reference(
                owner_id,
                storage.object_url(&format!("recipes/{victim_id}/{victim_object}_full.webp")),
            ),
            recipe_reference(
                owner_id,
                storage.object_url(&format!(
                    "recipes/{owner_id}-suffix/{victim_object}_full.webp"
                )),
            ),
        ];

        let referenced_keys = collect_retained_media_keys(&storage, &references);

        assert!(referenced_keys.contains(&format!("recipes/{owner_id}/{owned_object}_full.webp")));
        assert!(referenced_keys.contains(&format!("recipes/{owner_id}/{owned_object}_thumb.webp")));
        assert!(referenced_keys
            .iter()
            .all(|key| !key.contains(&victim_id.to_string())));
        assert_eq!(referenced_keys.len(), 2);
    }

    #[test]
    fn typed_references_retain_only_their_expected_namespace() {
        let owner_id = uuid::Uuid::new_v4();
        let recipe_object = uuid::Uuid::new_v4();
        let collection_object = uuid::Uuid::new_v4();
        let avatar_object = uuid::Uuid::new_v4();
        let storage = crate::storage::StorageClient::for_test();
        let references = vec![
            recipe_reference(
                owner_id,
                storage.object_url(&format!("recipes/{owner_id}/{recipe_object}_full.webp")),
            ),
            collection_reference(
                owner_id,
                storage.object_url(&format!(
                    "collections/{owner_id}/{collection_object}_full.webp"
                )),
            ),
            collection_reference(
                owner_id,
                storage.object_url(&format!("recipes/{owner_id}/{collection_object}_full.webp")),
            ),
            recipe_reference(
                owner_id,
                storage.object_url(&format!("collections/{owner_id}/{recipe_object}_full.webp")),
            ),
            collection_reference(
                owner_id,
                storage.object_url(&format!("avatars/{owner_id}/{avatar_object}.webp")),
            ),
        ];

        let retained = collect_retained_media_keys(&storage, &references);

        assert_eq!(retained.len(), 4);
        assert!(retained.contains(&format!("recipes/{owner_id}/{recipe_object}_full.webp")));
        assert!(retained.contains(&format!("recipes/{owner_id}/{recipe_object}_thumb.webp")));
        assert!(retained.contains(&format!(
            "collections/{owner_id}/{collection_object}_full.webp"
        )));
        assert!(retained.contains(&format!(
            "collections/{owner_id}/{collection_object}_thumb.webp"
        )));
    }

    #[test]
    fn inherited_owner_exception_never_applies_to_collection_media() {
        let collection_owner = uuid::Uuid::new_v4();
        let referencing_owner = uuid::Uuid::new_v4();
        let object_id = uuid::Uuid::new_v4();
        let storage = crate::storage::StorageClient::for_test();
        let mut reference = collection_reference(
            referencing_owner,
            storage.object_url(&format!(
                "collections/{collection_owner}/{object_id}_full.webp"
            )),
        );
        reference.allows_inherited_owner = true;

        assert!(collect_retained_media_keys(&storage, &[reference]).is_empty());
    }

    #[tokio::test]
    async fn cleanup_never_lists_the_untracked_storage_corpus() {
        let (_database, pool) = crate::test_utils::setup_test_db().await;
        let suffix = crate::test_utils::uid();
        let owner = crate::db::insert_user(
            &pool,
            &format!("bounded_{suffix}"),
            "Bounded Cleanup",
            &format!("bounded-{suffix}@example.test"),
            None,
        )
        .await
        .unwrap();
        let object_id = uuid::Uuid::new_v4();
        let mock_server = wiremock::MockServer::start().await;
        let storage = crate::storage::StorageClient::for_test_with_endpoint(&mock_server.uri());
        let full_key = format!("recipes/{}/{object_id}_full.webp", owner.id);
        let thumbnail_key = format!("recipes/{}/{object_id}_thumb.webp", owner.id);
        crate::db::register_media_upload_family(
            &pool,
            &crate::db::NewMediaUploadFamily {
                owner_id: owner.id,
                purpose: crate::db::MediaPurpose::Recipe,
                full_url: storage.object_url(&full_key),
                thumbnail_url: storage.object_url(&thumbnail_key),
                full_key,
                thumbnail_key,
                capability_digest: vec![61; 32],
                expires_at: chrono::Utc::now() - chrono::Duration::minutes(1),
            },
        )
        .await
        .unwrap();

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(200))
            .expect(0)
            .mount(&mock_server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("DELETE"))
            .respond_with(wiremock::ResponseTemplate::new(204))
            .expect(2)
            .mount(&mock_server)
            .await;

        run_cleanup(&pool, &storage).await.unwrap();
        mock_server.verify().await;
        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT lifecycle FROM media_upload_families WHERE owner_id = $1",
            )
            .bind(owner.id)
            .fetch_one(&pool)
            .await
            .unwrap(),
            "discarded"
        );
    }

    #[tokio::test]
    async fn account_media_cleanup_retries_after_the_user_is_gone() {
        let (_database, pool) = crate::test_utils::setup_test_db().await;
        let suffix = crate::test_utils::uid();
        let user = crate::db::insert_user(
            &pool,
            &format!("cleanup_retry_{suffix}"),
            "Cleanup Retry",
            &format!("cleanup-retry-{suffix}@example.test"),
            None,
        )
        .await
        .unwrap();
        let object_id = uuid::Uuid::new_v4();
        let mock_server = wiremock::MockServer::start().await;
        let storage = crate::storage::StorageClient::for_test_with_endpoint_and_public_url(
            &mock_server.uri(),
            "https://media.example.test",
        );
        let full_url = storage.object_url(&format!("avatars/{}/{object_id}.webp", user.id));
        let original_url = storage.object_url(&format!("avatars/{}/{object_id}-original", user.id));
        sqlx::query("UPDATE users SET avatar_url = $2, avatar_original_url = $3 WHERE id = $1")
            .bind(user.id)
            .bind(&full_url)
            .bind(&original_url)
            .execute(&pool)
            .await
            .unwrap();
        crate::db::delete_user_with_media_cleanup(&pool, user.id, &storage)
            .await
            .unwrap();

        wiremock::Mock::given(wiremock::matchers::method("DELETE"))
            .respond_with(wiremock::ResponseTemplate::new(500))
            .expect(3)
            .mount(&mock_server)
            .await;
        run_account_media_cleanup(&pool, &storage).await.unwrap();

        let (lifecycle, attempts, failure): (String, i32, Option<String>) = sqlx::query_as(
            "SELECT lifecycle, attempt_count, delete_failure \
             FROM account_media_cleanup_tombstones WHERE former_owner_id = $1",
        )
        .bind(user.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(lifecycle, "pending");
        assert_eq!(attempts, 1);
        assert!(failure.is_some());
        assert!(crate::db::get_user_by_id(&pool, user.id)
            .await
            .unwrap()
            .is_none());

        mock_server.reset().await;
        wiremock::Mock::given(wiremock::matchers::method("DELETE"))
            .respond_with(wiremock::ResponseTemplate::new(204))
            .expect(3)
            .mount(&mock_server)
            .await;
        run_account_media_cleanup(&pool, &storage).await.unwrap();

        let (lifecycle, attempts, failure): (String, i32, Option<String>) = sqlx::query_as(
            "SELECT lifecycle, attempt_count, delete_failure \
             FROM account_media_cleanup_tombstones WHERE former_owner_id = $1",
        )
        .bind(user.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(lifecycle, "deleted");
        assert_eq!(attempts, 2);
        assert!(failure.is_none());
    }
}
