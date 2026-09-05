//! Orphan image cleanup worker.
//!
//! Periodically reconciles a bounded batch of tracked media assets against
//! exact database references. Unknown legacy objects are never enumerated or
//! treated as deletion authority.

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
    // Candidate selection and each indexed reference check are independently
    // bounded. The bucket and document corpus are never enumerated.
    let candidates = db::list_media_asset_cleanup_candidates(pool).await?;
    let candidates_examined = candidates.len();
    let mut deleted_count = 0u64;
    for asset_id in candidates {
        let db::MediaDiscardDecision::Claimed(claim) =
            db::claim_media_cleanup(pool, asset_id).await?
        else {
            continue;
        };
        let mut failures = Vec::new();
        let mut keys = vec![claim.full_key, claim.thumbnail_key];
        keys.extend(claim.original_key);
        for key in &keys {
            if let Err(error) = storage.delete_object(key).await {
                failures.push(format!("{key}: {error}"));
            }
        }
        let failure = (!failures.is_empty()).then(|| failures.join("; "));
        db::finish_media_asset_delete(pool, claim.id, failure.as_deref()).await?;
        if let Some(error) = failure {
            warn!(claim_id = %claim.id, %error, "Failed to delete tracked orphan family");
        } else {
            deleted_count += keys.len() as u64;
            debug!(claim_id = %claim.id, "Deleted unreferenced media asset");
        }
    }

    info!(
        candidates_examined,
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
        crate::db::register_media_asset(
            &pool,
            &crate::db::NewMediaAsset {
                id: object_id,
                uploader_id: owner.id,
                purpose: crate::db::MediaPurpose::Recipe,
                full_url: storage.object_url(&full_key),
                thumbnail_url: storage.object_url(&thumbnail_key),
                full_key,
                thumbnail_key,
                original_url: None,
                original_key: None,
                capability_digest: Some(vec![61; 32]),
                expires_at: Some(chrono::Utc::now() - chrono::Duration::minutes(1)),
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
                "SELECT lifecycle FROM media_assets WHERE uploader_id = $1",
            )
            .bind(owner.id)
            .fetch_one(&pool)
            .await
            .unwrap(),
            "deleted"
        );
    }

    #[tokio::test]
    async fn media_asset_cleanup_retries_after_the_uploader_is_gone() {
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
        let full_key = format!("avatars/{}/{object_id}.webp", user.id);
        let thumbnail_key = format!("avatars/{}/{object_id}-thumb.webp", user.id);
        let original_key = format!("avatars/{}/{object_id}-original", user.id);
        let full_url = storage.object_url(&full_key);
        let thumbnail_url = storage.object_url(&thumbnail_key);
        let original_url = storage.object_url(&original_key);
        crate::db::register_media_asset(
            &pool,
            &crate::db::NewMediaAsset {
                id: object_id,
                uploader_id: user.id,
                purpose: crate::db::MediaPurpose::Avatar,
                full_url: full_url.clone(),
                thumbnail_url,
                original_url: Some(original_url.clone()),
                full_key,
                thumbnail_key,
                original_key: Some(original_key),
                capability_digest: None,
                expires_at: None,
            },
        )
        .await
        .unwrap();
        crate::db::replace_avatar_media_reference(&pool, user.id, object_id)
            .await
            .unwrap();
        sqlx::query("UPDATE users SET avatar_url = $2, avatar_original_url = $3 WHERE id = $1")
            .bind(user.id)
            .bind(&full_url)
            .bind(&original_url)
            .execute(&pool)
            .await
            .unwrap();
        crate::db::delete_user_with_media_cleanup(&pool, user.id)
            .await
            .unwrap();

        wiremock::Mock::given(wiremock::matchers::method("DELETE"))
            .respond_with(wiremock::ResponseTemplate::new(500))
            .expect(3)
            .mount(&mock_server)
            .await;
        run_cleanup(&pool, &storage).await.unwrap();

        let (lifecycle, failure): (String, Option<String>) =
            sqlx::query_as("SELECT lifecycle, delete_failure FROM media_assets WHERE id = $1")
                .bind(object_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(lifecycle, "delete_claimed");
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
        run_cleanup(&pool, &storage).await.unwrap();

        let (lifecycle, failure): (String, Option<String>) =
            sqlx::query_as("SELECT lifecycle, delete_failure FROM media_assets WHERE id = $1")
                .bind(object_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(lifecycle, "deleted");
        assert!(failure.is_none());
    }
}
