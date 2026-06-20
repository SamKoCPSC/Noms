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
    /// Read config from environment.
    ///
    /// `NOMS_ORPHAN_CLEANUP_INTERVAL_SECS` — interval in seconds (default: 3600 = 1 hour).
    pub fn from_env() -> Self {
        let interval_secs = std::env::var("NOMS_ORPHAN_CLEANUP_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(3600);
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
    info!(interval_secs = config.interval.as_secs(), "Spawning orphan cleanup worker");
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
    // 1. Get all referenced image URLs from DB
    let db_urls = db::get_all_image_urls(pool)
        .await
        .map_err(|e| format!("Failed to query image URLs from DB: {e}"))?;

    // 2. Build set of referenced storage keys
    let public_url = storage.public_url().trim_end_matches('/');
    let mut referenced_keys = HashSet::new();
    for url in &db_urls {
        if let Some(key) = url.strip_prefix(public_url).map(|s| s.trim_start_matches('/')) {
            referenced_keys.insert(key.to_string());
            // Also mark the derived thumbnail key as referenced so it isn't
            // deleted as an orphan (the DB stores only _full.webp URLs).
            if let Some(base) = key.strip_suffix("_full.webp") {
                referenced_keys.insert(format!("{base}_thumb.webp"));
            }
        }
    }

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
        objects_in_db = db_urls.len(),
        objects_in_storage = all_keys.len(),
        orphans_deleted = deleted_count,
        "Orphan cleanup cycle complete"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orphan_cleanup_config_default() {
        // Ensure the env var is not set
        std::env::remove_var("NOMS_ORPHAN_CLEANUP_INTERVAL_SECS");
        let config = OrphanCleanupConfig::from_env();
        assert_eq!(config.interval, Duration::from_secs(3600));
    }

    #[test]
    fn test_orphan_cleanup_config_from_env() {
        std::env::set_var("NOMS_ORPHAN_CLEANUP_INTERVAL_SECS", "42");
        let config = OrphanCleanupConfig::from_env();
        assert_eq!(config.interval, Duration::from_secs(42));
        // Clean up
        std::env::remove_var("NOMS_ORPHAN_CLEANUP_INTERVAL_SECS");
    }

    #[test]
    fn test_orphan_cleanup_config_invalid_env() {
        std::env::set_var("NOMS_ORPHAN_CLEANUP_INTERVAL_SECS", "not-a-number");
        let config = OrphanCleanupConfig::from_env();
        // Should fall back to default
        assert_eq!(config.interval, Duration::from_secs(3600));
        // Clean up
        std::env::remove_var("NOMS_ORPHAN_CLEANUP_INTERVAL_SECS");
    }

    #[test]
    fn test_thumbnail_keys_derived_from_full_keys() {
        // Simulate the key derivation that happens inside run_cleanup:
        // for every "_full.webp" key, the corresponding "_thumb.webp" is
        // also added to referenced_keys so thumbnails aren't deleted.
        let public_url = "https://cdn.example.com";
        let db_urls = vec![
            format!("{public_url}/recipes/user1/abc123_full.webp"),
            format!("{public_url}/recipes/user2/def456_full.webp"),
        ];

        let mut referenced_keys = HashSet::new();
        for url in &db_urls {
            if let Some(key) = url.strip_prefix(public_url).map(|s| s.trim_start_matches('/')) {
                referenced_keys.insert(key.to_string());
                if let Some(base) = key.strip_suffix("_full.webp") {
                    referenced_keys.insert(format!("{base}_thumb.webp"));
                }
            }
        }

        // Full keys are present
        assert!(referenced_keys.contains("recipes/user1/abc123_full.webp"));
        assert!(referenced_keys.contains("recipes/user2/def456_full.webp"));

        // Derived thumbnail keys are also present
        assert!(referenced_keys.contains("recipes/user1/abc123_thumb.webp"));
        assert!(referenced_keys.contains("recipes/user2/def456_thumb.webp"));

        // A genuine orphan (not matching any full or thumb) is not referenced
        assert!(!referenced_keys.contains("recipes/user3/orphan_full.webp"));
        assert!(!referenced_keys.contains("recipes/user3/orphan_thumb.webp"));

        // A non-image object is not referenced
        assert!(!referenced_keys.contains("recipes/user1/some_other_file.txt"));
    }

    #[test]
    fn test_thumbnail_derivation_ignores_non_full_keys() {
        // Keys that don't end with "_full.webp" should not produce spurious
        // thumbnail entries.
        let public_url = "https://cdn.example.com";
        let db_urls = vec![
            // This one has no _full suffix — no thumb should be derived
            format!("{public_url}/recipes/user1/legacy_image.webp"),
            // This one is a full key — thumb should be derived
            format!("{public_url}/recipes/user1/abc123_full.webp"),
        ];

        let mut referenced_keys = HashSet::new();
        for url in &db_urls {
            if let Some(key) = url.strip_prefix(public_url).map(|s| s.trim_start_matches('/')) {
                referenced_keys.insert(key.to_string());
                if let Some(base) = key.strip_suffix("_full.webp") {
                    referenced_keys.insert(format!("{base}_thumb.webp"));
                }
            }
        }

        assert!(referenced_keys.contains("recipes/user1/legacy_image.webp"));
        // No spurious thumb for the legacy key
        assert!(!referenced_keys.contains("recipes/user1/legacy_image_thumb.webp"));
        // Thumb for the full key is present
        assert!(referenced_keys.contains("recipes/user1/abc123_thumb.webp"));
    }
}
