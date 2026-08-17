//! File upload API: multipart handler and server function.
//!
//! `UploadResponse` and the `#[server] upload_image` function are compiled on
//! both targets — the `#[server]` macro strips the function body on the client
//! (so server-only references inside it are not type-checked client-side;
//! this is the same mechanism `recipe.rs` relies on). The `upload_handler`
//! Axum handler and the integration tests remain server-only.
//!
//! Server is a pass-through: accepts pre-processed WebP bytes from the client,
//! validates magic bytes, stores both variants, returns URLs.
#![cfg_attr(not(feature = "server"), allow(dead_code, unused_imports))]

#[cfg(feature = "server")]
use axum::extract::Multipart;
#[cfg(feature = "server")]
use axum::extract::{Extension, State};
#[cfg(feature = "server")]
use axum::http::StatusCode;
#[cfg(feature = "server")]
use axum::response::IntoResponse;
#[cfg(feature = "server")]
use axum::Json;
#[cfg(feature = "server")]
use axum_extra::extract::cookie::CookieJar;
use dioxus::prelude::*;
#[cfg(feature = "server")]
use std::sync::{Arc, OnceLock};
#[cfg(feature = "server")]
use tracing::{error, info};
#[cfg(feature = "server")]
use uuid::Uuid;

#[cfg(feature = "server")]
use crate::auth::session;

// -- Constants --

const MAX_FILE_SIZE: usize = 10 * 1024 * 1024; // 10 MB

/// Maximum encoded request body for server-function uploads containing both variants.
pub(crate) const SERVER_FUNCTION_BODY_LIMIT: usize = 24 * 1024 * 1024; // 24 MiB

#[cfg(feature = "server")]
pub(crate) fn server_function_body_limit_layer() -> axum::extract::DefaultBodyLimit {
    axum::extract::DefaultBodyLimit::max(SERVER_FUNCTION_BODY_LIMIT)
}

/// Validate WebP magic bytes: "RIFF....WEBP" (first 4 + bytes 8-12).
pub fn validate_webp_magic(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP"
}

#[cfg(feature = "server")]
fn validate_image_variants(
    full_bytes: &[u8],
    thumbnail_bytes: &[u8],
    maximum_full_size: usize,
    maximum_thumbnail_size: usize,
) -> Result<(), &'static str> {
    if full_bytes.is_empty() || thumbnail_bytes.is_empty() {
        return Err("Image variants must not be empty");
    }
    if full_bytes.len() > maximum_full_size || thumbnail_bytes.len() > maximum_thumbnail_size {
        return Err("Image variant exceeds the size limit");
    }
    Ok(())
}

#[cfg(feature = "server")]
pub(crate) async fn load_user_media_usage(
    storage: &crate::storage::StorageClient,
    owner_id: Uuid,
) -> Result<crate::security::MediaUsage, crate::storage::StorageError> {
    let mut aggregate = crate::security::MediaUsage::default();
    // These exact prefixes are the complete set of owner-controlled media.
    for namespace in ["recipes", "collections", "avatars"] {
        let prefix = format!("{namespace}/{owner_id}/");
        let remaining =
            crate::security::MAX_MEDIA_OBJECTS_PER_USER.saturating_sub(aggregate.objects);
        let usage = storage.media_usage(&prefix, remaining).await?;
        aggregate.bytes = aggregate.bytes.saturating_add(usage.bytes);
        aggregate.objects = aggregate.objects.saturating_add(usage.objects);
        if aggregate.objects > crate::security::MAX_MEDIA_OBJECTS_PER_USER {
            return Ok(aggregate);
        }
    }
    Ok(aggregate)
}

#[cfg(feature = "server")]
static MEDIA_UPLOAD_LOCKS: OnceLock<dashmap::DashMap<Uuid, Arc<tokio::sync::Mutex<()>>>> =
    OnceLock::new();

#[cfg(feature = "server")]
pub(crate) fn media_upload_lock(owner_id: Uuid) -> Arc<tokio::sync::Mutex<()>> {
    let locks = MEDIA_UPLOAD_LOCKS.get_or_init(dashmap::DashMap::new);
    if locks.len() > 4_096 {
        locks.retain(|_, lock| Arc::strong_count(lock) > 1);
    }
    locks
        .entry(owner_id)
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone()
}

#[cfg(feature = "server")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MediaQuotaCheckError {
    UsageUnavailable,
    Exceeded,
}

#[cfg(feature = "server")]
async fn enforce_media_upload_quota(
    storage: &crate::storage::StorageClient,
    owner_id: Uuid,
    incoming_bytes: usize,
    incoming_objects: u64,
) -> Result<(), MediaQuotaCheckError> {
    let current = load_user_media_usage(storage, owner_id)
        .await
        .map_err(|error| {
            tracing::error!(%error, %owner_id, "failed to load aggregate media usage");
            MediaQuotaCheckError::UsageUnavailable
        })?;
    let incoming = crate::security::MediaUsage {
        bytes: u64::try_from(incoming_bytes).unwrap_or(u64::MAX),
        objects: incoming_objects,
    };
    crate::security::validate_media_quota(current, incoming)
        .map_err(|_| MediaQuotaCheckError::Exceeded)
}

/// Response from image upload, containing both full-size and thumbnail URLs.
///
/// Explicit `thumb_url` is returned rather than derived via string replacement,
/// ensuring reliability for all upload scenarios.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UploadResponse {
    /// Full-size WebP image URL
    pub full_url: String,
    /// Thumbnail WebP image URL — explicitly computed, not derived
    pub thumb_url: String,
    /// Storage key for the full-size image
    pub key: String,
}

/// Closed set of media destinations exposed by the upload API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum UploadPurpose {
    Recipe,
    CollectionCover,
}

#[cfg(feature = "server")]
fn media_variant_keys(owner_id: Uuid, object_id: Uuid, purpose: UploadPurpose) -> (String, String) {
    let namespace = match purpose {
        UploadPurpose::Recipe => "recipes",
        UploadPurpose::CollectionCover => "collections",
    };
    (
        format!("{namespace}/{owner_id}/{object_id}_full.webp"),
        format!("{namespace}/{owner_id}/{object_id}_thumb.webp"),
    )
}

#[cfg(feature = "server")]
async fn upload_media_variants(
    storage: &crate::storage::StorageClient,
    owner_id: Uuid,
    full_bytes: &[u8],
    thumbnail_bytes: &[u8],
    purpose: UploadPurpose,
) -> Result<UploadResponse, crate::storage::StorageError> {
    let (full_key, thumbnail_key) = media_variant_keys(owner_id, Uuid::new_v4(), purpose);
    let full_url = storage
        .put_object(&full_key, full_bytes, "image/webp")
        .await?;
    let thumbnail_url = match storage
        .put_object(&thumbnail_key, thumbnail_bytes, "image/webp")
        .await
    {
        Ok(url) => url,
        Err(upload_error) => {
            // The full key was generated by this request and is not yet
            // referenceable, so it is the only safe compensation target.
            if let Err(delete_error) = storage.delete_object(&full_key).await {
                tracing::error!(
                    key = %full_key,
                    error = %delete_error,
                    "Failed to compensate partial media upload"
                );
            }
            return Err(upload_error);
        }
    };

    Ok(UploadResponse {
        full_url,
        thumb_url: thumbnail_url,
        key: full_key,
    })
}

// -- Server Function (for Dioxus frontend) --

/// Upload pre-processed image bytes and return both full-size and thumbnail URLs.
///
/// Called from the frontend. Accepts already-processed WebP bytes for both
/// the full-size image and the thumbnail. Validates WebP magic bytes, stores
/// both variants, and returns the public URLs.
#[server]
pub async fn upload_image(
    full_bytes: Vec<u8>,
    thumb_bytes: Vec<u8>,
    filename: String,
    purpose: UploadPurpose,
) -> Result<UploadResponse, ServerFnError> {
    let _ = filename; // kept for future extension extraction
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    validate_image_variants(&full_bytes, &thumb_bytes, MAX_FILE_SIZE, MAX_FILE_SIZE)
        .map_err(ServerFnError::new)?;

    // Validate WebP magic bytes (defense-in-depth)
    if !validate_webp_magic(&full_bytes) {
        return Err(ServerFnError::new(
            "Invalid full-size image. Expected WebP format.",
        ));
    }
    if !validate_webp_magic(&thumb_bytes) {
        return Err(ServerFnError::new(
            "Invalid thumbnail. Expected WebP format.",
        ));
    }

    // Store both variants via global storage client
    let Some(storage) = crate::storage::try_get_storage_client() else {
        return Err(ServerFnError::new("Image storage is unavailable"));
    };
    crate::security::enforce_user_request_budget(user_id, crate::security::RequestClass::Upload)
        .map_err(|_| ServerFnError::new("Upload rate limit exceeded"))?;
    let upload_lock = media_upload_lock(user_id);
    let _guard = upload_lock.lock().await;
    enforce_media_upload_quota(
        &storage,
        user_id,
        full_bytes.len().saturating_add(thumb_bytes.len()),
        2,
    )
    .await
    .map_err(|error| match error {
        MediaQuotaCheckError::UsageUnavailable => {
            ServerFnError::new("Unable to verify media quota")
        }
        MediaQuotaCheckError::Exceeded => ServerFnError::new("Media storage quota exceeded"),
    })?;
    let response = upload_media_variants(&storage, user_id, &full_bytes, &thumb_bytes, purpose)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, user_id = %user_id, "Failed to upload media variants");
            ServerFnError::new("Failed to upload file")
        })?;
    tracing::info!(full_key = %response.key, user_id = %user_id, "Uploaded image variants");
    Ok(response)
}

/// Delete an image from S3 storage given its full URL.
///
/// Parses the exact server-generated storage URL, then deletes both the
/// full-size and thumbnail objects. Best-effort: if the full object is already
/// gone, the thumbnail deletion is still attempted.
///
/// Auth-gated: only the authenticated user can delete images. Ownership is
/// enforced by exact UUID namespace and filename validation.
#[server]
pub async fn delete_image(full_url: String) -> Result<(), ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    crate::security::enforce_user_request_budget(user_id, crate::security::RequestClass::Mutation)
        .map_err(|_| ServerFnError::new("Mutation rate limit exceeded"))?;

    let Some(storage) = crate::storage::try_get_storage_client() else {
        return Err(ServerFnError::new("Image storage is unavailable"));
    };
    let (full_key, thumb_key) = recipe_deletion_keys(&storage, user_id, &full_url)?;

    // Delete full-size object (best-effort: ignore "not found")
    let _ = storage.delete_object(&full_key).await;

    // Delete thumbnail object (best-effort: ignore "not found")
    let _ = storage.delete_object(&thumb_key).await;

    Ok(())
}

/// Delete an uploaded collection cover that was removed before it was saved.
#[server]
pub async fn delete_unsaved_collection_cover(full_url: String) -> Result<(), ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    crate::security::enforce_user_request_budget(user_id, crate::security::RequestClass::Mutation)
        .map_err(|_| ServerFnError::new("Mutation rate limit exceeded"))?;
    let Some(storage) = crate::storage::try_get_storage_client() else {
        return Err(ServerFnError::new("Image storage is unavailable"));
    };
    let (full_key, thumbnail_key) = collection_cover_deletion_keys(&storage, user_id, &full_url)?;
    let _ = storage.delete_object(&full_key).await;
    let _ = storage.delete_object(&thumbnail_key).await;
    Ok(())
}

#[cfg(feature = "server")]
fn recipe_deletion_keys(
    storage: &crate::storage::StorageClient,
    owner_id: uuid::Uuid,
    full_url: &str,
) -> Result<(String, String), ServerFnError> {
    let object = storage
        .parse_media_url(full_url)
        .map_err(ServerFnError::new)?;
    if object.owner_id() != owner_id || object.kind() != crate::storage::MediaObjectKind::RecipeFull
    {
        return Err(ServerFnError::new(
            "Access denied: image does not belong to this user",
        ));
    }
    let thumbnail_key = object
        .paired_thumbnail_key()
        .ok_or_else(|| ServerFnError::new("Invalid full-size recipe image URL"))?;
    Ok((object.key().to_string(), thumbnail_key))
}

#[cfg(feature = "server")]
fn collection_cover_deletion_keys(
    storage: &crate::storage::StorageClient,
    owner_id: uuid::Uuid,
    full_url: &str,
) -> Result<(String, String), ServerFnError> {
    let object = storage
        .parse_media_url(full_url)
        .map_err(ServerFnError::new)?;
    if object.owner_id() != owner_id
        || object.kind() != crate::storage::MediaObjectKind::CollectionFull
    {
        return Err(ServerFnError::new(
            "Access denied: collection cover does not belong to this user",
        ));
    }
    let thumbnail_key = object
        .paired_thumbnail_key()
        .ok_or_else(|| ServerFnError::new("Invalid full-size collection cover URL"))?;
    Ok((object.key().to_string(), thumbnail_key))
}

// -- Axum Handler (for direct API / fetch access) --

/// Handle POST /api/upload.
///
/// Accepts multipart form data with TWO fields: `full_image` and `thumb_image`.
/// Both must be valid WebP files. Validates magic bytes, uploads to
/// S3-compatible storage, and returns the public URLs.
#[cfg(feature = "server")]
pub async fn upload_handler(
    State(state): State<crate::auth::user_profile::UserProfileState>,
    Extension(storage): Extension<crate::storage::StorageClient>,
    jar: CookieJar,
    mut multipart: Multipart,
) -> axum::response::Response {
    // Verify session -- returns Uuid (user_id) directly
    let configured_cookie_name = session::cookie_name();
    let session_token = jar.get(configured_cookie_name.as_ref());
    let user_id = if let Some(cookie) = session_token {
        match session::verify_session(&state.pool, cookie.value()).await {
            Ok(id) => Some(id),
            Err(e) => {
                tracing::warn!(error = %e, "Session verification failed");
                None
            }
        }
    } else {
        None
    };

    let user_id = match user_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                "Authentication required".to_string(),
            )
                .into_response();
        }
    };

    // Parse multipart form -- extract `full_image` and `thumb_image` fields
    let mut full_bytes: Option<Vec<u8>> = None;
    let mut thumb_bytes: Option<Vec<u8>> = None;

    while let Some(field) = match multipart.next_field().await {
        Ok(f) => f,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("Failed to parse multipart: {e}"),
            )
                .into_response();
        }
    } {
        let name = field.name().map(String::from);

        let data = match field.bytes().await {
            Ok(d) => d,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    format!("Failed to read field: {e}"),
                )
                    .into_response();
            }
        };

        // Check size early
        if data.len() > MAX_FILE_SIZE {
            return (
                StatusCode::PAYLOAD_TOO_LARGE,
                "File too large. Maximum size is 10MB.".to_string(),
            )
                .into_response();
        }

        match name.as_deref() {
            Some("full_image") => full_bytes = Some(data.to_vec()),
            Some("thumb_image") => thumb_bytes = Some(data.to_vec()),
            _ => continue,
        }
    }

    let full_bytes = match full_bytes {
        Some(b) => b,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                "Missing 'full_image' field.".to_string(),
            )
                .into_response();
        }
    };

    let thumb_bytes = match thumb_bytes {
        Some(b) => b,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                "Missing 'thumb_image' field.".to_string(),
            )
                .into_response();
        }
    };

    if let Err(message) =
        validate_image_variants(&full_bytes, &thumb_bytes, MAX_FILE_SIZE, MAX_FILE_SIZE)
    {
        return (StatusCode::BAD_REQUEST, message.to_string()).into_response();
    }

    // Validate WebP magic bytes (defense-in-depth)
    if !validate_webp_magic(&full_bytes) {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            "Invalid full_image. Expected WebP format.".to_string(),
        )
            .into_response();
    }
    if !validate_webp_magic(&thumb_bytes) {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            "Invalid thumb_image. Expected WebP format.".to_string(),
        )
            .into_response();
    }

    if crate::security::enforce_user_request_budget(user_id, crate::security::RequestClass::Upload)
        .is_err()
    {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            "Upload rate limit exceeded".to_string(),
        )
            .into_response();
    }
    let upload_lock = media_upload_lock(user_id);
    let _guard = upload_lock.lock().await;
    if let Err(error) = enforce_media_upload_quota(
        &storage,
        user_id,
        full_bytes.len().saturating_add(thumb_bytes.len()),
        2,
    )
    .await
    {
        if error == MediaQuotaCheckError::Exceeded {
            return (
                StatusCode::PAYLOAD_TOO_LARGE,
                "Media storage quota exceeded".to_string(),
            )
                .into_response();
        }
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to upload file".to_string(),
        )
            .into_response();
    }

    // The direct multipart endpoint remains recipe-specific. Collection
    // covers use the typed full-stack server function above.
    let response = match upload_media_variants(
        &storage,
        user_id,
        &full_bytes,
        &thumb_bytes,
        UploadPurpose::Recipe,
    )
    .await
    {
        Ok(response) => response,
        Err(e) => {
            error!(error = %e, user_id = %user_id, "Failed to upload media variants");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to upload file".to_string(),
            )
                .into_response();
        }
    };

    info!(full_key = %response.key, user_id = %user_id, "Uploaded image variants");

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "full_url": response.full_url,
            "thumb_url": response.thumb_url,
            "key": response.key
        })),
    )
        .into_response()
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(all(feature = "server", test))]
mod tests {
    use super::*;
    use crate::test_utils;
    use axum::body::Body as AxumBody;
    use axum::http::Request;
    use axum::Router;
    use serde_json::Value;
    use tower::ServiceExt;

    const TEST_SECRET: &str = "test-secret-32-bytes-long-enough!!";
    const BOUNDARY: &str = "boundary123456789";

    /// Minimal valid WebP file bytes (RIFF....WEBP header).
    /// 12 bytes: "RIFF" + 4-byte little-endian file size + "WEBP".
    const VALID_WEBP_BYTES: &[u8] = b"RIFF\x24\x00\x00\x00WEBP";

    // ── Helpers ─────────────────────────────────────────────────────────────

    fn setup_test_env() {
        session::set_test_secret(TEST_SECRET);
    }

    async fn create_test_user(pool: &sqlx::PgPool) -> uuid::Uuid {
        let u = test_utils::uid();
        let user = crate::db::insert_user(
            pool,
            &format!("uploader_{u}"),
            "Upload Test User",
            &format!("upload{u}@example.com"),
            None,
        )
        .await
        .expect("failed to create test user");
        user.id
    }

    async fn create_session_token(pool: &sqlx::PgPool, user_id: uuid::Uuid) -> String {
        session::create_session(pool, user_id)
            .await
            .expect("failed to create session token")
    }

    async fn make_router(pool: sqlx::PgPool) -> Router {
        make_router_with_storage(pool, crate::storage::StorageClient::for_test()).await
    }

    async fn make_router_with_storage(
        pool: sqlx::PgPool,
        storage: crate::storage::StorageClient,
    ) -> Router {
        Router::new()
            .route("/api/upload", axum::routing::post(upload_handler))
            .layer(axum::Extension(storage))
            .with_state(crate::auth::user_profile::UserProfileState { pool })
    }

    /// Build a multipart form request with `full_image` and `thumb_image` fields.
    fn build_multipart_request(
        full_bytes: &[u8],
        thumb_bytes: &[u8],
        session_cookie: Option<&str>,
    ) -> Request<AxumBody> {
        let full_str = std::str::from_utf8(full_bytes).unwrap_or("");
        let thumb_str = std::str::from_utf8(thumb_bytes).unwrap_or("");

        let body = format!(
            "--{boundary}\r\n\
             Content-Disposition: form-data; name=\"full_image\"; filename=\"full.webp\"\r\n\
             Content-Type: image/webp\r\n\r\n\
             {full}\r\n\
             --{boundary}\r\n\
             Content-Disposition: form-data; name=\"thumb_image\"; filename=\"thumb.webp\"\r\n\
             Content-Type: image/webp\r\n\r\n\
             {thumb}\r\n\
             --{boundary}--\r\n",
            boundary = BOUNDARY,
            full = full_str,
            thumb = thumb_str,
        );

        let mut builder = Request::builder().method("POST").uri("/api/upload").header(
            "Content-Type",
            format!("multipart/form-data; boundary={BOUNDARY}"),
        );

        if let Some(cookie) = session_cookie {
            builder = builder.header("Cookie", format!("{}={}", session::COOKIE_NAME, cookie));
        }

        builder.body(AxumBody::from(body)).unwrap()
    }

    /// Build a multipart request missing the `full_image` field.
    fn build_multipart_no_full(session_cookie: Option<&str>) -> Request<AxumBody> {
        let body = format!(
            "--{boundary}\r\n\
             Content-Disposition: form-data; name=\"thumb_image\"; filename=\"thumb.webp\"\r\n\
             Content-Type: image/webp\r\n\r\n\
             thumb-data\r\n\
             --{boundary}--",
            boundary = BOUNDARY,
        );

        let mut builder = Request::builder().method("POST").uri("/api/upload").header(
            "Content-Type",
            format!("multipart/form-data; boundary={BOUNDARY}"),
        );

        if let Some(cookie) = session_cookie {
            builder = builder.header("Cookie", format!("{}={}", session::COOKIE_NAME, cookie));
        }

        builder.body(AxumBody::from(body)).unwrap()
    }

    /// Build a multipart request missing the `thumb_image` field.
    fn build_multipart_no_thumb(session_cookie: Option<&str>) -> Request<AxumBody> {
        let body = format!(
            "--{boundary}\r\n\
             Content-Disposition: form-data; name=\"full_image\"; filename=\"full.webp\"\r\n\
             Content-Type: image/webp\r\n\r\n\
             full-data\r\n\
             --{boundary}--",
            boundary = BOUNDARY,
        );

        let mut builder = Request::builder().method("POST").uri("/api/upload").header(
            "Content-Type",
            format!("multipart/form-data; boundary={BOUNDARY}"),
        );

        if let Some(cookie) = session_cookie {
            builder = builder.header("Cookie", format!("{}={}", session::COOKIE_NAME, cookie));
        }

        builder.body(AxumBody::from(body)).unwrap()
    }

    /// Read response body as bytes.
    async fn read_body(response: axum::response::Response) -> Vec<u8> {
        axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec()
    }

    /// Read response body as JSON Value.
    async fn read_json(response: axum::response::Response) -> Value {
        let bytes = read_body(response).await;
        serde_json::from_slice(&bytes).expect("response should be valid JSON")
    }

    async fn mock_empty_user_media_usage(mock_server: &wiremock::MockServer) {
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?><ListBucketResult><Name>noms-media</Name><KeyCount>0</KeyCount><MaxKeys>1000</MaxKeys><IsTruncated>false</IsTruncated></ListBucketResult>",
            ))
            .expect(3)
            .mount(mock_server)
            .await;
    }

    // ── WebP magic byte validation: see `validate_webp_magic_tests` at file
    //    scope (S3) — the pure tests run on both targets, including wasm.
    //    Tests below exercise the magic-byte check via the HTTP handler.

    // ── Auth flow tests ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_upload_no_session_returns_401() {
        setup_test_env();
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool).await;

        let response = app
            .oneshot(build_multipart_request(
                VALID_WEBP_BYTES,
                VALID_WEBP_BYTES,
                None,
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let body_bytes = read_body(response).await;
        let body_str = String::from_utf8_lossy(&body_bytes);
        assert!(
            body_str.contains("Authentication required"),
            "Response body: {body_str}"
        );
    }

    #[tokio::test]
    async fn test_upload_invalid_session_returns_401() {
        setup_test_env();
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool).await;

        let response = app
            .oneshot(build_multipart_request(
                VALID_WEBP_BYTES,
                VALID_WEBP_BYTES,
                Some("invalid-token-that-is-not-a-jwt"),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let body_bytes = read_body(response).await;
        let body_str = String::from_utf8_lossy(&body_bytes);
        assert!(
            body_str.contains("Authentication required"),
            "Response body: {body_str}"
        );
    }

    #[tokio::test]
    async fn test_upload_missing_full_field_returns_400() {
        setup_test_env();
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool.clone()).await;
        let user_id = create_test_user(&pool).await;
        let token = create_session_token(&pool, user_id).await;

        let response = app
            .oneshot(build_multipart_no_full(Some(&token)))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body_bytes = read_body(response).await;
        let body_str = String::from_utf8_lossy(&body_bytes);
        assert!(
            body_str.contains("Missing 'full_image'"),
            "Response body: {body_str}"
        );
    }

    #[tokio::test]
    async fn test_upload_missing_thumb_field_returns_400() {
        setup_test_env();
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool.clone()).await;
        let user_id = create_test_user(&pool).await;
        let token = create_session_token(&pool, user_id).await;

        let response = app
            .oneshot(build_multipart_no_thumb(Some(&token)))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body_bytes = read_body(response).await;
        let body_str = String::from_utf8_lossy(&body_bytes);
        assert!(
            body_str.contains("Missing 'thumb_image'"),
            "Response body: {body_str}"
        );
    }

    // ── WebP validation tests ───────────────────────────────────────────────

    #[tokio::test]
    async fn test_upload_invalid_webp_full_rejected_with_422() {
        setup_test_env();
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool.clone()).await;
        let user_id = create_test_user(&pool).await;
        let token = create_session_token(&pool, user_id).await;

        // Invalid WebP in full_image field
        let response = app
            .oneshot(build_multipart_request(
                b"this-is-not-webp-at-all",
                VALID_WEBP_BYTES,
                Some(&token),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let body_bytes = read_body(response).await;
        let body_str = String::from_utf8_lossy(&body_bytes);
        assert!(
            body_str.contains("Invalid full_image") || body_str.contains("WebP"),
            "Response body: {body_str}"
        );
    }

    #[tokio::test]
    async fn test_upload_invalid_webp_thumb_rejected_with_422() {
        setup_test_env();
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool.clone()).await;
        let user_id = create_test_user(&pool).await;
        let token = create_session_token(&pool, user_id).await;

        // Invalid WebP in thumb_image field
        let response = app
            .oneshot(build_multipart_request(
                VALID_WEBP_BYTES,
                b"this-is-not-webp-at-all",
                Some(&token),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let body_bytes = read_body(response).await;
        let body_str = String::from_utf8_lossy(&body_bytes);
        assert!(
            body_str.contains("Invalid thumb_image") || body_str.contains("WebP"),
            "Response body: {body_str}"
        );
    }

    #[tokio::test]
    async fn test_upload_valid_webp_passes_validation() {
        setup_test_env();
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool.clone()).await;
        let user_id = create_test_user(&pool).await;
        let token = create_session_token(&pool, user_id).await;

        let response = app
            .oneshot(build_multipart_request(
                VALID_WEBP_BYTES,
                VALID_WEBP_BYTES,
                Some(&token),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(read_body(response).await, b"Failed to upload file");
    }

    // ── Size validation tests ───────────────────────────────────────────────

    #[tokio::test]
    async fn test_upload_small_file_accepted() {
        setup_test_env();
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool.clone()).await;
        let user_id = create_test_user(&pool).await;
        let token = create_session_token(&pool, user_id).await;

        // 1 KB of data (not valid WebP, but size check comes first)
        let small_data = vec![0xFFu8; 1024];

        let response = app
            .oneshot(build_multipart_request(
                &small_data,
                &small_data,
                Some(&token),
            ))
            .await
            .unwrap();

        // Size check passes (not 413). WebP validation or storage may fail.
        assert!(
            response.status() != StatusCode::PAYLOAD_TOO_LARGE,
            "1KB file should pass size validation, got {} with body: {}",
            response.status(),
            String::from_utf8_lossy(&read_body(response).await)
        );
    }

    #[tokio::test]
    async fn test_upload_over_limit_rejected_with_413() {
        setup_test_env();
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool.clone()).await;
        let user_id = create_test_user(&pool).await;
        let token = create_session_token(&pool, user_id).await;

        // 11 MB of data — exceeds the 10 MB limit
        let large_data = vec![b'X'; 11 * 1024 * 1024];

        let response = app
            .oneshot(build_multipart_request(
                &large_data,
                VALID_WEBP_BYTES,
                Some(&token),
            ))
            .await
            .unwrap();

        let status = response.status();
        let body_bytes = read_body(response).await;
        let body_str = String::from_utf8_lossy(&body_bytes);

        // Accept 413 (handler size check) or 400 (multipart parser buffer
        // limit). Both indicate the oversized file was rejected.
        assert!(
            status == StatusCode::PAYLOAD_TOO_LARGE || status == StatusCode::BAD_REQUEST,
            "Expected 413 or 400 for oversized file, got {} with body: {}",
            status,
            body_str
        );
    }

    #[tokio::test]
    async fn server_function_body_limit_accepts_a_near_limit_request() {
        assert!(
            super::SERVER_FUNCTION_BODY_LIMIT >= (MAX_FILE_SIZE * 2) + (64 * 1024),
            "server-function body limit must include both max-size variants and encoding overhead"
        );

        let app = Router::new()
            .route(
                "/",
                axum::routing::post(|_: axum::body::Bytes| async { StatusCode::NO_CONTENT }),
            )
            .layer(super::server_function_body_limit_layer());
        let request_body = vec![0u8; super::SERVER_FUNCTION_BODY_LIMIT - 1];
        let response = app
            .oneshot(
                Request::post("/")
                    .body(AxumBody::from(request_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    // ── Filename sanitization tests ─────────────────────────────────────────

    #[tokio::test]
    async fn test_upload_path_traversal_sanitized() {
        setup_test_env();
        let (_db, pool) = test_utils::setup_test_db().await;
        let mock_server = wiremock::MockServer::start().await;
        mock_empty_user_media_usage(&mock_server).await;
        wiremock::Mock::given(wiremock::matchers::method("PUT"))
            .respond_with(wiremock::ResponseTemplate::new(200))
            .expect(2)
            .mount(&mock_server)
            .await;
        let app = make_router_with_storage(
            pool.clone(),
            crate::storage::StorageClient::for_test_with_endpoint(&mock_server.uri()),
        )
        .await;
        let user_id = create_test_user(&pool).await;
        let token = create_session_token(&pool, user_id).await;

        // Build request with path traversal in filename
        let webp_str = std::str::from_utf8(VALID_WEBP_BYTES).unwrap();
        let body = format!(
            "--{boundary}\r\n\
             Content-Disposition: form-data; name=\"full_image\"; filename=\"../../../etc/passwd.webp\"\r\n\
             Content-Type: image/webp\r\n\r\n\
             {webp}\r\n\
             --{boundary}\r\n\
             Content-Disposition: form-data; name=\"thumb_image\"; filename=\"thumb.webp\"\r\n\
             Content-Type: image/webp\r\n\r\n\
             {webp}\r\n\
             --{boundary}--\r\n",
            boundary = BOUNDARY,
            webp = webp_str,
        );

        let request = Request::builder()
            .method("POST")
            .uri("/api/upload")
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={BOUNDARY}"),
            )
            .header("Cookie", format!("{}={}", session::COOKIE_NAME, token))
            .body(AxumBody::from(body))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = read_json(response).await;
        let key = json["key"].as_str().expect("key should be a string");
        assert!(key.starts_with(&format!("recipes/{user_id}/")));
        assert!(!key.contains(".."));
        assert!(!key.contains("etc/passwd"));
    }

    #[tokio::test]
    async fn test_upload_null_byte_sanitized() {
        setup_test_env();
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool.clone()).await;
        let user_id = create_test_user(&pool).await;
        let token = create_session_token(&pool, user_id).await;

        // Build request with null byte in filename
        let webp_str = std::str::from_utf8(VALID_WEBP_BYTES).unwrap();
        let null_byte_filename = "test\x00.webp";
        let body = format!(
            "--{boundary}\r\n\
             Content-Disposition: form-data; name=\"full_image\"; filename=\"{name}\"\r\n\
             Content-Type: image/webp\r\n\r\n\
             {webp}\r\n\
             --{boundary}\r\n\
             Content-Disposition: form-data; name=\"thumb_image\"; filename=\"thumb.webp\"\r\n\
             Content-Type: image/webp\r\n\r\n\
             {webp}\r\n\
             --{boundary}--\r\n",
            boundary = BOUNDARY,
            name = null_byte_filename,
            webp = webp_str,
        );

        let request = Request::builder()
            .method("POST")
            .uri("/api/upload")
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={BOUNDARY}"),
            )
            .header("Cookie", format!("{}={}", session::COOKIE_NAME, token))
            .body(AxumBody::from(body))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        let status = response.status();
        let body_bytes = read_body(response).await;
        let body_str = String::from_utf8_lossy(&body_bytes);

        // The handler should not crash. Null bytes in filenames may be
        // rejected by the multipart parser (400) or the HTTP layer.
        // In any case, it should not be a successful upload (200).
        assert!(
            status != StatusCode::OK,
            "Null byte in filename should not result in successful upload, got {} with body: {}",
            status,
            body_str
        );
    }

    // ── Storage interaction test ────────────────────────────────────────────

    /// Full success path: valid auth + valid WebP -> 200 with full_url, thumb_url, and key.
    ///
    /// Uses a request-scoped storage client pointed at a mock S3 server.
    #[tokio::test]
    async fn test_upload_handler_success() {
        setup_test_env();

        // Start a wiremock server to intercept S3 calls
        let mock_server = wiremock::MockServer::start().await;
        let endpoint = mock_server.uri();
        mock_empty_user_media_usage(&mock_server).await;

        // Mock the S3 PutObject call — accept any PUT request (we expect 2: full + thumb)
        let mock = wiremock::Mock::given(wiremock::matchers::method("PUT"))
            .respond_with(wiremock::ResponseTemplate::new(200));
        mock.expect(2).mount(&mock_server).await;

        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router_with_storage(
            pool.clone(),
            crate::storage::StorageClient::for_test_with_endpoint(&endpoint),
        )
        .await;
        let user_id = create_test_user(&pool).await;
        let token = create_session_token(&pool, user_id).await;

        let response = app
            .oneshot(build_multipart_request(
                VALID_WEBP_BYTES,
                VALID_WEBP_BYTES,
                Some(&token),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let json = read_json(response).await;

        // Verify response contains full_url, thumb_url, and key
        assert!(
            json.get("full_url").is_some(),
            "Response should contain 'full_url'"
        );
        assert!(
            json.get("thumb_url").is_some(),
            "Response should contain 'thumb_url'"
        );
        assert!(json.get("key").is_some(), "Response should contain 'key'");

        // Verify the key format: recipes/{user_id}/{uuid}_full.webp
        let key = json["key"].as_str().expect("key should be a string");
        let parts: Vec<&str> = key.split('/').collect();
        assert_eq!(
            parts.len(),
            3,
            "Key should have 3 segments: recipes/user_id/filename"
        );
        assert_eq!(parts[0], "recipes");
        assert_eq!(parts[1], user_id.to_string());

        // Filename should end with _full.webp
        assert!(
            parts[2].ends_with("_full.webp"),
            "Filename should end with _full.webp, got: {}",
            parts[2]
        );

        // UUID part (before _full.webp) should be a valid UUID format
        let uuid_str = parts[2].strip_suffix("_full.webp").unwrap();
        uuid::Uuid::parse_str(uuid_str).expect("UUID part of key should be valid");

        // Verify thumb_url contains _thumb.webp
        let thumb_url = json["thumb_url"]
            .as_str()
            .expect("thumb_url should be a string");
        assert!(
            thumb_url.contains("_thumb.webp"),
            "thumb_url should contain '_thumb.webp', got: {}",
            thumb_url
        );
    }

    #[test]
    fn upload_purpose_selects_a_closed_storage_namespace() {
        let owner_id = uuid::Uuid::new_v4();
        let object_id = uuid::Uuid::new_v4();

        assert_eq!(
            media_variant_keys(owner_id, object_id, UploadPurpose::Recipe),
            (
                format!("recipes/{owner_id}/{object_id}_full.webp"),
                format!("recipes/{owner_id}/{object_id}_thumb.webp")
            )
        );
        assert_eq!(
            media_variant_keys(owner_id, object_id, UploadPurpose::CollectionCover),
            (
                format!("collections/{owner_id}/{object_id}_full.webp"),
                format!("collections/{owner_id}/{object_id}_thumb.webp")
            )
        );
    }

    #[test]
    fn upload_variants_reject_empty_and_accept_exact_size_boundaries() {
        let mut boundary = vec![0_u8; MAX_FILE_SIZE];
        boundary[0..4].copy_from_slice(b"RIFF");
        boundary[8..12].copy_from_slice(b"WEBP");

        assert!(
            validate_image_variants(&boundary, &boundary, MAX_FILE_SIZE, MAX_FILE_SIZE).is_ok()
        );
        assert!(
            validate_image_variants(&[], VALID_WEBP_BYTES, MAX_FILE_SIZE, MAX_FILE_SIZE).is_err()
        );
        assert!(
            validate_image_variants(VALID_WEBP_BYTES, &[], MAX_FILE_SIZE, MAX_FILE_SIZE).is_err()
        );

        boundary.push(0);
        assert!(
            validate_image_variants(&boundary, VALID_WEBP_BYTES, MAX_FILE_SIZE, MAX_FILE_SIZE)
                .is_err()
        );
    }

    #[tokio::test]
    async fn media_usage_sums_recipe_collection_and_avatar_namespaces() {
        let owner_id = uuid::Uuid::new_v4();
        let mock_server = wiremock::MockServer::start().await;
        let storage = crate::storage::StorageClient::for_test_with_endpoint(&mock_server.uri());

        for (namespace, size) in [("recipes", 11), ("collections", 13), ("avatars", 17)] {
            let key = format!("{namespace}/{owner_id}/{}.webp", uuid::Uuid::new_v4());
            wiremock::Mock::given(wiremock::matchers::method("GET"))
                .and(wiremock::matchers::query_param(
                    "prefix",
                    format!("{namespace}/{owner_id}/"),
                ))
                .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(format!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?><ListBucketResult><Name>noms-media</Name><KeyCount>1</KeyCount><MaxKeys>1000</MaxKeys><IsTruncated>false</IsTruncated><Contents><Key>{key}</Key><Size>{size}</Size></Contents></ListBucketResult>"
                )))
                .expect(1)
                .mount(&mock_server)
                .await;
        }

        assert_eq!(
            load_user_media_usage(&storage, owner_id).await.unwrap(),
            crate::security::MediaUsage {
                bytes: 41,
                objects: 3,
            }
        );
    }

    #[tokio::test]
    async fn partial_collection_upload_removes_only_the_generated_full_object() {
        let owner_id = uuid::Uuid::new_v4();
        let mock_server = wiremock::MockServer::start().await;
        let storage = crate::storage::StorageClient::for_test_with_endpoint(&mock_server.uri());

        wiremock::Mock::given(wiremock::matchers::method("PUT"))
            .and(wiremock::matchers::path_regex(
                r"/noms-media/collections/.+_full\.webp$",
            ))
            .respond_with(wiremock::ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("PUT"))
            .and(wiremock::matchers::path_regex(
                r"/noms-media/collections/.+_thumb\.webp$",
            ))
            .respond_with(wiremock::ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("DELETE"))
            .and(wiremock::matchers::path_regex(
                r"/noms-media/collections/.+_full\.webp$",
            ))
            .respond_with(wiremock::ResponseTemplate::new(204))
            .expect(1)
            .mount(&mock_server)
            .await;

        let result = upload_media_variants(
            &storage,
            owner_id,
            VALID_WEBP_BYTES,
            VALID_WEBP_BYTES,
            UploadPurpose::CollectionCover,
        )
        .await;

        assert!(result.is_err());
    }

    #[test]
    fn direct_delete_requires_an_exact_owned_full_image_key() {
        let owner_id = uuid::Uuid::new_v4();
        let victim_id = uuid::Uuid::new_v4();
        let object_id = uuid::Uuid::new_v4();
        let storage = crate::storage::StorageClient::for_test();
        let owned_url = storage.object_url(&format!("recipes/{owner_id}/{object_id}_full.webp"));

        assert_eq!(
            recipe_deletion_keys(&storage, owner_id, &owned_url).unwrap(),
            (
                format!("recipes/{owner_id}/{object_id}_full.webp"),
                format!("recipes/{owner_id}/{object_id}_thumb.webp")
            )
        );

        for unsafe_url in [
            storage.object_url(&format!("recipes/{victim_id}/{object_id}_full.webp")),
            storage.object_url(&format!("recipes/{owner_id}-suffix/{object_id}_full.webp")),
            storage.object_url(&format!("recipes/{owner_id}/../{object_id}_full.webp")),
            storage.object_url(&format!("avatars/{owner_id}/{object_id}.webp")),
        ] {
            assert!(
                recipe_deletion_keys(&storage, owner_id, &unsafe_url).is_err(),
                "unsafe direct deletion URL was accepted: {unsafe_url}"
            );
        }
    }

    #[test]
    fn unsaved_collection_cover_delete_requires_an_exact_owned_collection_key() {
        let owner_id = uuid::Uuid::new_v4();
        let victim_id = uuid::Uuid::new_v4();
        let object_id = uuid::Uuid::new_v4();
        let storage = crate::storage::StorageClient::for_test();
        let owned_url =
            storage.object_url(&format!("collections/{owner_id}/{object_id}_full.webp"));

        assert_eq!(
            collection_cover_deletion_keys(&storage, owner_id, &owned_url).unwrap(),
            (
                format!("collections/{owner_id}/{object_id}_full.webp"),
                format!("collections/{owner_id}/{object_id}_thumb.webp")
            )
        );

        for unsafe_url in [
            storage.object_url(&format!("collections/{victim_id}/{object_id}_full.webp")),
            storage.object_url(&format!("recipes/{owner_id}/{object_id}_full.webp")),
            storage.object_url(&format!("avatars/{owner_id}/{object_id}.webp")),
        ] {
            assert!(
                collection_cover_deletion_keys(&storage, owner_id, &unsafe_url).is_err(),
                "unsafe collection deletion URL was accepted: {unsafe_url}"
            );
        }
    }
}

// ── Pure `validate_webp_magic` tests (cross-platform — S3) ─────────────────
//
// Split out of the server-gated `tests` module so they also run on the
// wasm32 target. The validator is pure Rust (no server deps), so it is
// safe to compile and run anywhere.

#[cfg(test)]
mod validate_webp_magic_tests {
    use super::validate_webp_magic;

    /// Minimal valid WebP file bytes (RIFF....WEBP header).
    /// 12 bytes: "RIFF" + 4-byte little-endian file size + "WEBP".
    const VALID_WEBP_BYTES: &[u8] = b"RIFF\x24\x00\x00\x00WEBP";

    #[test]
    fn test_validate_webp_magic_valid() {
        assert!(validate_webp_magic(VALID_WEBP_BYTES));
    }

    #[test]
    fn test_validate_webp_magic_too_short() {
        assert!(!validate_webp_magic(b"RIFF"));
    }

    #[test]
    fn test_validate_webp_magic_bad_header() {
        assert!(!validate_webp_magic(b"NOTR....WEBPxxxx"));
    }

    #[test]
    fn test_validate_webp_magic_bad_footer() {
        assert!(!validate_webp_magic(b"RIFF....JPEGxxxx"));
    }

    #[test]
    fn test_validate_webp_magic_jpeg_rejected() {
        // JPEG starts with 0xFFD8
        assert!(!validate_webp_magic(&[
            0xFF, 0xD8, 0xFF, 0xE0, 0, 0, 0, 0, 0, 0, 0, 0
        ]));
    }

    #[test]
    fn test_validate_webp_magic_png_rejected() {
        // PNG starts with 0x89 0x50 0x4E 0x47
        assert!(!validate_webp_magic(&[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0
        ]));
    }
}
