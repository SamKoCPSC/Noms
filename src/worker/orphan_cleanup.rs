//! Orphan image cleanup worker.
//!
//! Periodically reconciles tracked upload families against database references.
//! Storage listings are preflight/observability only; unknown legacy objects
//! are never treated as deletion authority.

#[cfg(test)]
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
    // Reference inventory remains useful observability, but deletion authority
    // comes exclusively from a locked, tracked upload-family claim.
    let db_references = db::get_all_owned_image_references(pool)
        .await
        .map_err(|e| format!("Failed to query image URLs from DB: {e}"))?;

    // Complete both namespace preflights before any mutation. Listings are
    // observability only: unknown legacy objects never become delete targets.
    let recipe_keys = storage
        .list_objects(Some("recipes/"))
        .await
        .map_err(|e| format!("Failed to list recipe storage objects: {e}"))?;
    let collection_keys = storage
        .list_objects(Some("collections/"))
        .await
        .map_err(|e| format!("Failed to list collection storage objects: {e}"))?;

    let candidates = db::list_media_cleanup_candidates(pool, 500).await?;
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
        objects_in_db = db_references.len(),
        objects_in_storage = recipe_keys.len() + collection_keys.len(),
        orphans_deleted = deleted_count,
        "Orphan cleanup cycle complete"
    );
    Ok(())
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
    async fn cleanup_preflights_both_namespaces_before_any_deletion() {
        let (_database, pool) = crate::test_utils::setup_test_db().await;
        let owner_id = uuid::Uuid::new_v4();
        let object_id = uuid::Uuid::new_v4();
        let recipe_key = format!("recipes/{owner_id}/{object_id}_full.webp");
        let mock_server = wiremock::MockServer::start().await;
        let storage = crate::storage::StorageClient::for_test_with_endpoint(&mock_server.uri());

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::query_param("prefix", "recipes/"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?><ListBucketResult><Name>noms-media</Name><KeyCount>1</KeyCount><MaxKeys>1000</MaxKeys><IsTruncated>false</IsTruncated><Contents><Key>{recipe_key}</Key></Contents></ListBucketResult>"
            )))
            .expect(1)
            .mount(&mock_server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::query_param("prefix", "collections/"))
            .respond_with(wiremock::ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("DELETE"))
            .respond_with(wiremock::ResponseTemplate::new(204))
            .expect(0)
            .mount(&mock_server)
            .await;

        assert!(run_cleanup(&pool, &storage).await.is_err());
    }
}
