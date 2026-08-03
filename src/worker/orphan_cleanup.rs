//! Orphan image cleanup worker.
//!
//! Periodically reconciles MinIO storage against the database:
//! 1. Collects all image URLs referenced by recipes in the DB
//! 2. Lists all objects in the storage bucket
//! 3. Deletes any object whose key is not referenced by the DB

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
/// The task runs on a periodic interval, querying all image URLs from the DB,
/// listing all objects in storage, and deleting any orphans.
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
    // 1. Get every image URL together with its authorized owner context.
    let db_references = db::get_all_owned_image_references(pool)
        .await
        .map_err(|e| format!("Failed to query image URLs from DB: {e}"))?;

    // 2. Retain only canonical recipe keys whose embedded owner matches the
    // recipe owner. A foreign URL in recipe JSON is not a retention grant.
    let referenced_keys = collect_owned_recipe_keys(storage, &db_references);

    // 3. List all objects in storage (recipes/ prefix)
    let all_keys = storage
        .list_objects(Some("recipes/"))
        .await
        .map_err(|e| format!("Failed to list storage objects: {e}"))?;

    // 4. Delete orphans
    let mut deleted_count = 0u64;
    for key in &all_keys {
        if !referenced_keys.contains(key.as_str()) {
            if let Err(e) = storage.delete_object(key).await {
                warn!(key = %key, error = %e, "Failed to delete orphan object");
            } else {
                deleted_count += 1;
                debug!(key = %key, "Deleted orphan object");
            }
        }
    }

    info!(
        objects_in_db = db_references.len(),
        objects_in_storage = all_keys.len(),
        orphans_deleted = deleted_count,
        "Orphan cleanup cycle complete"
    );
    Ok(())
}

pub(crate) fn collect_owned_recipe_keys(
    storage: &StorageClient,
    references: &[db::OwnedImageReference],
) -> HashSet<String> {
    let mut keys = HashSet::new();
    for reference in references {
        let Ok(object) = storage.parse_media_url(&reference.image_url) else {
            continue;
        };
        // Only server-created fork drafts carry this exception. Their source
        // URLs are copied by the server, and autosave rejects any new foreign
        // internal URL, so the embedded object owner remains authoritative if
        // the source recipe is deleted before the draft is published.
        if object.owner_id() != reference.owner_id && !reference.allows_inherited_owner {
            continue;
        }
        match object.kind() {
            crate::storage::MediaObjectKind::RecipeFull => {
                keys.insert(object.key().to_string());
                if let Some(thumbnail_key) = object.paired_thumbnail_key() {
                    keys.insert(thumbnail_key);
                }
            }
            crate::storage::MediaObjectKind::RecipeThumbnail => {
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

    fn owned_reference(owner_id: uuid::Uuid, image_url: String) -> db::OwnedImageReference {
        db::OwnedImageReference {
            owner_id,
            image_url,
            allows_inherited_owner: false,
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
            owned_reference(
                first_owner,
                storage.object_url(&format!("recipes/{first_owner}/{first_object}_full.webp")),
            ),
            owned_reference(
                second_owner,
                storage.object_url(&format!("recipes/{second_owner}/{second_object}_full.webp")),
            ),
        ];

        let referenced_keys = collect_owned_recipe_keys(&storage, &references);

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
        let references = vec![owned_reference(
            owner_id,
            storage.object_url(&format!("recipes/{owner_id}/legacy_image.webp")),
        )];

        assert!(collect_owned_recipe_keys(&storage, &references).is_empty());
    }

    #[test]
    fn foreign_or_malformed_recipe_references_do_not_retain_objects() {
        let owner_id = uuid::Uuid::new_v4();
        let victim_id = uuid::Uuid::new_v4();
        let owned_object = uuid::Uuid::new_v4();
        let victim_object = uuid::Uuid::new_v4();
        let storage = crate::storage::StorageClient::for_test();
        let references = vec![
            owned_reference(
                owner_id,
                storage.object_url(&format!("recipes/{owner_id}/{owned_object}_full.webp")),
            ),
            owned_reference(
                owner_id,
                storage.object_url(&format!("recipes/{victim_id}/{victim_object}_full.webp")),
            ),
            owned_reference(
                owner_id,
                storage.object_url(&format!(
                    "recipes/{owner_id}-suffix/{victim_object}_full.webp"
                )),
            ),
        ];

        let referenced_keys = collect_owned_recipe_keys(&storage, &references);

        assert!(referenced_keys.contains(&format!("recipes/{owner_id}/{owned_object}_full.webp")));
        assert!(referenced_keys.contains(&format!("recipes/{owner_id}/{owned_object}_thumb.webp")));
        assert!(referenced_keys
            .iter()
            .all(|key| !key.contains(&victim_id.to_string())));
        assert_eq!(referenced_keys.len(), 2);
    }
}
