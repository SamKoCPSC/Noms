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
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use tokio::sync::OnceCell;

/// Errors from storage operations.
#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum StorageError {
    /// Failed to upload an object to the storage backend.
    UploadFailed(String),
    /// Failed to delete an object from the storage backend.
    DeleteFailed(String),
    /// Failed to list objects in the storage backend.
    ListFailed(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::UploadFailed(e) => write!(f, "storage upload failed: {e}"),
            StorageError::DeleteFailed(e) => write!(f, "storage delete failed: {e}"),
            StorageError::ListFailed(e) => write!(f, "storage list failed: {e}"),
        }
    }
}

impl std::error::Error for StorageError {}

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
        Self::for_test_with_endpoint("http://127.0.0.1:9")
    }

    /// Create a test client pointed at a specific mock endpoint.
    ///
    /// Retries are disabled so an unavailable endpoint fails immediately and
    /// handler tests remain deterministic and independent of network access.
    #[cfg(test)]
    pub fn for_test_with_endpoint(endpoint: &str) -> Self {
        let s3_config = aws_sdk_s3::config::Builder::new()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("us-east-1"))
            .endpoint_url(endpoint)
            .credentials_provider(Credentials::for_tests())
            .retry_config(aws_config::retry::RetryConfig::disabled())
            .build();
        Self {
            inner: S3Client::from_conf(s3_config),
            bucket: "noms-media".to_string(),
            public_url: "http://localhost:9000/noms-media".to_string(),
        }
    }

    /// Upload bytes to the configured bucket and return the public URL.
    pub async fn put_object(
        &self,
        key: &str,
        bytes: &[u8],
        content_type: &str,
    ) -> Result<String, StorageError> {
        self.inner
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .body(ByteStream::from(bytes.to_vec()))
            .send()
            .await
            .map_err(|e| StorageError::UploadFailed(e.to_string()))?;
        Ok(self.object_url(key))
    }

    /// Delete an object from the configured bucket by key.
    pub async fn delete_object(&self, key: &str) -> Result<(), StorageError> {
        self.inner
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| StorageError::DeleteFailed(e.to_string()))?;
        Ok(())
    }

    /// List all object keys in the bucket, optionally filtered by prefix.
    ///
    /// Uses the S3 `ListObjectsV2` paginator to handle pagination automatically.
    pub async fn list_objects(&self, prefix: Option<&str>) -> Result<Vec<String>, StorageError> {
        let mut builder = self.inner.list_objects_v2().bucket(&self.bucket);
        if let Some(p) = prefix {
            builder = builder.prefix(p);
        }
        let mut stream = builder.into_paginator().send();
        let mut keys = Vec::new();
        while let Some(page) = stream.next().await {
            match page {
                Ok(resp) => {
                    for obj in resp.contents() {
                        if let Some(key) = obj.key() {
                            keys.push(key.to_string());
                        }
                    }
                }
                Err(e) => return Err(StorageError::ListFailed(e.to_string())),
            }
        }
        Ok(keys)
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

/// Return the global client when storage has been initialized.
///
/// Request handlers use this non-panicking accessor so a startup or test
/// configuration error becomes a controlled response instead of aborting the
/// request task.
pub fn try_get_storage_client() -> Option<StorageClient> {
    STORAGE_CLIENT.get().cloned()
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

        let client = StorageClient::from_env()
            .await
            .expect("from_env should succeed");
        client
            .health_check()
            .await
            .expect("health check should succeed against running Minio");
    }
}
