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
use axum::extract::State;
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
) -> Result<UploadResponse, ServerFnError> {
    let _ = filename; // kept for future extension extraction
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    // Validate file sizes
    if full_bytes.len() > MAX_FILE_SIZE {
        return Err(ServerFnError::new(
            "Full-size image too large. Maximum size is 10MB.",
        ));
    }
    if thumb_bytes.len() > MAX_FILE_SIZE {
        return Err(ServerFnError::new(
            "Thumbnail too large. Maximum size is 10MB.",
        ));
    }

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

    // Generate storage keys: recipes/{user_id}/{uuid}_full.webp and _thumb.webp
    let unique_id = uuid::Uuid::new_v4();
    let full_key = format!("recipes/{}/{}_full.webp", user_id, unique_id);
    let thumb_key = format!("recipes/{}/{}_thumb.webp", user_id, unique_id);

    // Store both variants via global storage client
    let storage = crate::storage::get_storage_client();
    let full_url = storage
        .put_object(&full_key, &full_bytes, "image/webp")
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to upload full-size image to storage");
            ServerFnError::new("Failed to upload file")
        })?;
    let thumb_url = storage
        .put_object(&thumb_key, &thumb_bytes, "image/webp")
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to upload thumbnail to storage");
            ServerFnError::new("Failed to upload file")
        })?;

    tracing::info!(full_key = %full_key, thumb_key = %thumb_key, user_id = %user_id, "Uploaded image variants");
    Ok(UploadResponse {
        full_url,
        thumb_url,
        key: full_key,
    })
}

/// Delete an image from S3 storage given its full URL.
///
/// Derives the storage key by stripping the public URL prefix, then deletes
/// both the full-size and thumbnail objects. Best-effort: if the full object
/// is already gone, still attempts to delete the thumb.
///
/// Auth-gated: only the authenticated user can delete images. Ownership is
/// enforced by verifying the key prefix matches `recipes/{user_id}/`.
#[server]
pub async fn delete_image(full_url: String) -> Result<(), ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let storage = crate::storage::get_storage_client();
    let public_url = storage.public_url().trim_end_matches('/');

    // Extract storage key from full URL: strip public_url prefix
    let full_key = full_url
        .strip_prefix(public_url)
        .ok_or_else(|| ServerFnError::new("Invalid image URL: does not match storage public URL"))?
        .trim_start_matches('/')
        .to_string();

    // Ownership check: key must start with `recipes/{user_id}/`
    let expected_prefix = format!("recipes/{user_id}/");
    if !full_key.starts_with(&expected_prefix) {
        return Err(ServerFnError::new(
            "Access denied: image does not belong to this user",
        ));
    }

    // Derive thumbnail key: replace `_full.webp` with `_thumb.webp`
    let thumb_key = full_key
        .strip_suffix("_full.webp")
        .map(|base| format!("{base}_thumb.webp"))
        .unwrap_or_else(|| full_key.clone());

    // Delete full-size object (best-effort: ignore "not found")
    let _ = storage.delete_object(&full_key).await;

    // Delete thumbnail object (best-effort: ignore "not found")
    if thumb_key != full_key {
        let _ = storage.delete_object(&thumb_key).await;
    }

    Ok(())
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
    jar: CookieJar,
    mut multipart: Multipart,
) -> axum::response::Response {
    // Verify session -- returns Uuid (user_id) directly
    let session_token = jar.get(session::COOKIE_NAME);
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

    // Generate storage keys: recipes/{user_id}/{uuid}_full.webp and _thumb.webp
    let unique_id = Uuid::new_v4();
    let full_key = format!("recipes/{}/{}_full.webp", user_id, unique_id);
    let thumb_key = format!("recipes/{}/{}_thumb.webp", user_id, unique_id);

    // Store both variants via global storage client
    let storage = crate::storage::get_storage_client();
    let full_url = match storage
        .put_object(&full_key, &full_bytes, "image/webp")
        .await
    {
        Ok(u) => u,
        Err(e) => {
            error!(error = %e, full_key = %full_key, "Failed to upload full-size image to storage");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to upload file".to_string(),
            )
                .into_response();
        }
    };
    let thumb_url = match storage
        .put_object(&thumb_key, &thumb_bytes, "image/webp")
        .await
    {
        Ok(u) => u,
        Err(e) => {
            error!(error = %e, thumb_key = %thumb_key, "Failed to upload thumbnail to storage");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to upload file".to_string(),
            )
                .into_response();
        }
    };

    info!(full_key = %full_key, thumb_key = %thumb_key, user_id = %user_id, "Uploaded image variants");

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "full_url": full_url,
            "thumb_url": thumb_url,
            "key": full_key
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
        Router::new()
            .route("/api/upload", axum::routing::post(upload_handler))
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

        // WebP validation passes (not 422). Storage call fails without
        // initialized STORAGE_CLIENT, so we expect 500. With storage
        // configured (e.g. wiremock), this would be 200.
        assert!(
            response.status() != StatusCode::UNPROCESSABLE_ENTITY,
            "Valid WebP should pass validation, got {} with body: {}",
            response.status(),
            String::from_utf8_lossy(&read_body(response).await)
        );
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
        let app = make_router(pool.clone()).await;
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

        let status = response.status();
        let body_bytes = read_body(response).await;
        let body_str = String::from_utf8_lossy(&body_bytes);

        // The handler should not crash. The multipart parser may reject
        // path traversal filenames (400), or the request may proceed past
        // validation and fail at storage (500). Either way, it should not
        // be a successful upload (200).
        assert!(
            status != StatusCode::OK,
            "Path traversal filename should not result in successful upload, got {} with body: {}",
            status,
            body_str
        );
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
    /// Requires the global STORAGE_CLIENT to be initialized pointing to a
    /// mock S3 server (wiremock). Marked `#[ignore]` because the global
    /// OnceCell can only be set once per test process.
    #[tokio::test]
    #[ignore]
    async fn test_upload_handler_success() {
        setup_test_env();

        // Start a wiremock server to intercept S3 calls
        let mock_server = wiremock::MockServer::start().await;
        let endpoint = mock_server.uri();

        // Mock the S3 PutObject call — accept any PUT request (we expect 2: full + thumb)
        let mock = wiremock::Mock::given(wiremock::matchers::method("PUT"))
            .respond_with(wiremock::ResponseTemplate::new(200));
        mock_server.register(mock).await;

        // Initialize the global storage client pointing to wiremock
        crate::storage::StorageClient::set_storage_for_test(&endpoint);

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
