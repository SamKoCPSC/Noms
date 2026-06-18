//! S3-compatible storage client for media files.
//!
//! Wraps `aws_sdk_s3::Client` with environment-based configuration
//! and a global singleton initialized at server startup.
//!
//! Only compiled when the `server` feature is enabled.
#![cfg(feature = "server")]
// Public API items are used by other modules at runtime, but the compiler
// can't see that from this crate's binary target alone.
#![allow(dead_code)]

use aws_config::BehaviorVersion;
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::config::Region;
use aws_sdk_s3::Client as S3Client;
use tokio::sync::OnceCell;

// ── Storage client ──────────────────────────────────────────────────────────

/// S3-compatible storage client wrapping the AWS SDK S3 client.
///
/// Holds the configured S3 client, bucket name, and public base URL
/// for constructing object URLs.
#[derive(Clone)]
pub struct StorageClient {
    inner: S3Client,
    bucket: String,
    public_url: String,
}

impl std::fmt::Debug for StorageClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorageClient")
            .field("bucket", &self.bucket)
            .field("public_url", &self.public_url)
            .finish_non_exhaustive()
    }
}

impl StorageClient {
    /// Create a new `StorageClient` from environment variables.
    ///
    /// Reads the following environment variables (all required):
    /// - `R2_ENDPOINT` — R2-compatible endpoint URL (e.g., `http://localhost:9000`)
    /// - `R2_ACCESS_KEY_ID` — Access key ID
    /// - `R2_SECRET_ACCESS_KEY` — Secret access key
    /// - `R2_BUCKET` — Bucket name for media storage
    /// - `R2_PUBLIC_URL` — Public base URL for constructing object URLs
    ///
    /// Panics if any required environment variable is missing.
    pub async fn from_env() -> Result<Self, String> {
        let endpoint = std::env::var("R2_ENDPOINT")
            .map_err(|_| "R2_ENDPOINT environment variable is not set".to_string())?;
        let access_key = std::env::var("R2_ACCESS_KEY_ID")
            .map_err(|_| "R2_ACCESS_KEY_ID environment variable is not set".to_string())?;
        let secret_key = std::env::var("R2_SECRET_ACCESS_KEY")
            .map_err(|_| "R2_SECRET_ACCESS_KEY environment variable is not set".to_string())?;
        let bucket = std::env::var("R2_BUCKET")
            .map_err(|_| "R2_BUCKET environment variable is not set".to_string())?;
        let public_url = std::env::var("R2_PUBLIC_URL")
            .map_err(|_| "R2_PUBLIC_URL environment variable is not set".to_string())?;

        let region = Region::new("us-east-1");

        let shared_config = aws_config::defaults(BehaviorVersion::latest())
            .region(region.clone())
            .endpoint_url(endpoint.clone())
            .credentials_provider(Credentials::new(
                access_key,
                secret_key,
                None,
                None,
                "storage-client",
            ))
            .load()
            .await;

        let config = aws_sdk_s3::config::Builder::from(&shared_config)
            .force_path_style(true)
            .region(region)
            .build();

        let inner = S3Client::from_conf(config);

        Ok(Self {
            inner,
            bucket,
            public_url,
        })
    }

    /// Health check: verify connectivity by issuing a HeadBucket request.
    pub async fn health_check(&self) -> Result<(), String> {
        self.inner
            .head_bucket()
            .bucket(&self.bucket)
            .send()
            .await
            .map(|_| ())
            .map_err(|e| format!("S3 health check failed: {e}"))
    }

    /// Construct a public URL for an object key.
    ///
    /// Returns `{public_url}/{key}` with no double slashes.
    pub fn object_url(&self, key: &str) -> String {
        let base = self.public_url.trim_end_matches('/');
        let k = key.trim_start_matches('/');
        format!("{base}/{k}")
    }

    /// Return the configured bucket name.
    pub fn bucket(&self) -> &str {
        &self.bucket
    }

    /// Return the configured public base URL.
    pub fn public_url(&self) -> &str {
        &self.public_url
    }

    /// Create a minimal `StorageClient` for testing without environment variables.
    #[cfg(test)]
    pub fn for_test() -> Self {
        let s3_config = aws_sdk_s3::config::Builder::new()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("us-east-1"))
            .credentials_provider(Credentials::for_tests())
            .build();
        Self {
            inner: S3Client::from_conf(s3_config),
            bucket: "noms-media".to_string(),
            public_url: "http://localhost:9000/noms-media".to_string(),
        }
    }
}

// ── Global singleton ────────────────────────────────────────────────────────

/// Global storage client instance, initialized lazily on first access.
static STORAGE_CLIENT: OnceCell<StorageClient> = OnceCell::const_new();

/// Initialize the global storage client. Call once during application startup.
pub async fn init_storage() {
    let client = StorageClient::from_env()
        .await
        .expect("Failed to create storage client from environment");
    STORAGE_CLIENT
        .set(client)
        .expect("Storage client already initialized");
}

/// Get a clone of the global storage client.
///
/// Panics if the client has not been initialized via [`init_storage`].
pub fn get_storage_client() -> StorageClient {
    STORAGE_CLIENT
        .get()
        .expect("Storage client not initialized. Call storage::init_storage() first.")
        .clone()
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_client(public_url: &str, bucket: &str) -> StorageClient {
        // Build a minimal S3 config synchronously for unit tests.
        // No actual network calls are made — this is just for URL construction tests.
        let s3_config = aws_sdk_s3::config::Builder::new()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("us-east-1"))
            .credentials_provider(Credentials::for_tests())
            .build();
        StorageClient {
            inner: S3Client::from_conf(s3_config),
            bucket: bucket.to_string(),
            public_url: public_url.to_string(),
        }
    }

    #[test]
    fn test_object_url_simple_key() {
        let client = make_test_client("http://localhost:9000/noms-media", "noms-media");

        assert_eq!(
            client.object_url("images/recipe-123.jpg"),
            "http://localhost:9000/noms-media/images/recipe-123.jpg"
        );
    }

    #[test]
    fn test_object_url_key_with_leading_slash() {
        let client = make_test_client("http://localhost:9000/noms-media", "noms-media");

        assert_eq!(
            client.object_url("/images/recipe-123.jpg"),
            "http://localhost:9000/noms-media/images/recipe-123.jpg"
        );
    }

    #[test]
    fn test_object_url_public_url_with_trailing_slash() {
        let client = make_test_client("http://localhost:9000/noms-media/", "noms-media");

        assert_eq!(
            client.object_url("images/recipe-123.jpg"),
            "http://localhost:9000/noms-media/images/recipe-123.jpg"
        );
    }

    #[test]
    fn test_object_url_nested_path() {
        let client = make_test_client("http://localhost:9000/noms-media", "noms-media");

        assert_eq!(
            client.object_url("users/abc123/avatars/profile.jpg"),
            "http://localhost:9000/noms-media/users/abc123/avatars/profile.jpg"
        );
    }

    #[test]
    fn test_bucket_accessor() {
        let client = make_test_client("http://example.com", "test-bucket");

        assert_eq!(client.bucket(), "test-bucket");
    }

    #[test]
    fn test_public_url_accessor() {
        let client = make_test_client("http://example.com/bucket", "test-bucket");

        assert_eq!(client.public_url(), "http://example.com/bucket");
    }

    /// Integration test: requires a running S3-compatible server (Minio).
    #[tokio::test]
    #[ignore]
    async fn test_health_check_success() {
        std::env::set_var("R2_ENDPOINT", "http://localhost:9000");
        std::env::set_var("R2_ACCESS_KEY_ID", "noms-local-key");
        std::env::set_var("R2_SECRET_ACCESS_KEY", "noms-local-secret");
        std::env::set_var("R2_BUCKET", "noms-media");
        std::env::set_var("R2_PUBLIC_URL", "http://localhost:9000/noms-media");

        let client = StorageClient::from_env().await.expect("from_env should succeed");
        client
            .health_check()
            .await
            .expect("health check should succeed against running Minio");
    }
}
