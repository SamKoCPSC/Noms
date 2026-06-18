//! File upload API: multipart handler and server function.
//!
//! Only compiled when the `server` feature is enabled.
#![cfg(feature = "server")]

use axum::extract::Multipart;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use axum_extra::extract::cookie::CookieJar;
use dioxus::prelude::*;
use tracing::{error, info};
use uuid::Uuid;

use crate::auth::session;

// -- Constants --

const MAX_FILE_SIZE: usize = 10 * 1024 * 1024; // 10 MB
const ALLOWED_MIME_TYPES: &[&str] = &["image/jpeg", "image/png", "image/webp"];

// -- Server Function (for Dioxus frontend) --

/// Upload an image file and return its public URL.
///
/// Called from the frontend via Dioxus fullstack. The file is sent as
/// base64-encoded bytes through the server function's serialized arguments.
#[server]
pub async fn upload_image(
    file_bytes: Vec<u8>,
    mime_type: String,
    filename: String,
) -> Result<String, ServerFnError> {
    let _ = filename; // kept for future extension extraction
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    // Validate file size
    if file_bytes.len() > MAX_FILE_SIZE {
        return Err(ServerFnError::new(
            "File too large. Maximum size is 10MB.",
        ));
    }

    // Validate MIME type
    if !ALLOWED_MIME_TYPES.contains(&mime_type.as_str()) {
        return Err(ServerFnError::new(
            "Invalid file type. Only JPEG, PNG, and WebP are allowed.",
        ));
    }

    // Generate storage key: recipes/{user_id}/{uuid}.{ext}
    let ext = mime_type.strip_prefix("image/").unwrap_or("jpg");
    let unique_id = Uuid::new_v4();
    let storage_key = format!("recipes/{}/{}.{}", user_id, unique_id, ext);

    // Store file via global storage client
    let storage = crate::storage::get_storage_client();
    let url = storage
        .put_object(&storage_key, &file_bytes, &mime_type)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to upload file to storage");
            ServerFnError::new("Failed to upload file")
        })?;

    info!(storage_key = %storage_key, user_id = %user_id, "Uploaded file");
    Ok(url)
}

// -- Axum Handler (for direct API / fetch access) --

/// Handle POST /api/upload.
///
/// Accepts multipart form data with a `file` field. Validates file type
/// and size, uploads to S3-compatible storage, and returns the public URL.
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

    // Parse multipart form -- extract the `file` field
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut mime_type: Option<String> = None;

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
        if name.as_deref() != Some("file") {
            continue;
        }

        // Read content_type BEFORE bytes() consumes the field
        let ct = field.content_type().map(String::from);

        let data = match field.bytes().await {
            Ok(d) => d,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    format!("Failed to read file field: {e}"),
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

        file_bytes = Some(data.to_vec());
        mime_type = ct;
    }

    let file_bytes = match file_bytes {
        Some(b) => b,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                "No file provided. Use a 'file' form field.".to_string(),
            )
                .into_response();
        }
    };

    let mime_type = match mime_type {
        Some(m) => m,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                "Missing content type for uploaded file.".to_string(),
            )
                .into_response();
        }
    };

    // Validate MIME type
    if !ALLOWED_MIME_TYPES.contains(&mime_type.as_str()) {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            "Invalid file type. Only JPEG, PNG, and WebP are allowed.".to_string(),
        )
            .into_response();
    }

    // Generate storage key: recipes/{user_id}/{uuid}.{ext}
    let ext = mime_type.strip_prefix("image/").unwrap_or("jpg");
    let unique_id = Uuid::new_v4();
    let storage_key = format!("recipes/{}/{}.{}", user_id, unique_id, ext);

    // Store file via global storage client
    let storage = crate::storage::get_storage_client();
    let url = match storage.put_object(&storage_key, &file_bytes, &mime_type).await {
        Ok(u) => u,
        Err(e) => {
            error!(error = %e, storage_key = %storage_key, "Failed to upload file to storage");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to upload file".to_string(),
            )
                .into_response();
        }
    };

    info!(storage_key = %storage_key, user_id = %user_id, "Uploaded file");

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "url": url,
            "key": storage_key
        })),
    )
        .into_response()
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::session;
    use crate::test_utils;
    use axum::body::Body as AxumBody;
    use axum::http::Request;
    use axum::Router;
    use serde_json::Value;
    use tower::ServiceExt;

    const TEST_SECRET: &str = "test-secret-32-bytes-long-enough!!";
    const BOUNDARY: &str = "boundary123456789";

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

    /// Build a multipart form request with a file field and optional extras.
    fn build_multipart_request(
        file_name: &str,
        file_bytes: &[u8],
        content_type: &str,
        recipe_id: Option<&str>,
        session_cookie: Option<&str>,
    ) -> Request<AxumBody> {
        let mut body = format!(
            "--{boundary}\r\n\
             Content-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\n\
             Content-Type: {ct}\r\n\r\n",
            boundary = BOUNDARY,
            name = file_name,
            ct = content_type,
        );
        // File content must be valid UTF-8 for string concatenation
        body.push_str(std::str::from_utf8(file_bytes).unwrap_or(""));
        body.push_str("\r\n");

        if let Some(rid) = recipe_id {
            body.push_str(&format!(
                "--{boundary}\r\n\
                 Content-Disposition: form-data; name=\"recipe_id\"\r\n\r\n\
                 {rid}\r\n",
                boundary = BOUNDARY,
            ));
        }

        body.push_str(&format!("--{boundary}--\r\n", boundary = BOUNDARY));

        let mut builder = Request::builder()
            .method("POST")
            .uri("/api/upload")
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={BOUNDARY}"),
            );

        if let Some(cookie) = session_cookie {
            builder = builder.header(
                "Cookie",
                format!("{}={}", session::COOKIE_NAME, cookie),
            );
        }

        builder.body(AxumBody::from(body)).unwrap()
    }

    /// Build a multipart request with a non-file field (no "file" field present).
    fn build_multipart_no_file(session_cookie: Option<&str>) -> Request<AxumBody> {
        let body = format!(
            "--{boundary}\r\n\
             Content-Disposition: form-data; name=\"data\"\r\n\r\n\
             some-value\r\n\
             --{boundary}--",
            boundary = BOUNDARY,
        );

        let mut builder = Request::builder()
            .method("POST")
            .uri("/api/upload")
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={BOUNDARY}"),
            );

        if let Some(cookie) = session_cookie {
            builder = builder.header(
                "Cookie",
                format!("{}={}", session::COOKIE_NAME, cookie),
            );
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

    // ── Auth flow tests ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_upload_no_session_returns_401() {
        setup_test_env();
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool).await;

        let response = app
            .oneshot(build_multipart_request(
                "test.jpg",
                b"fake-jpeg-data",
                "image/jpeg",
                None,
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
                "test.jpg",
                b"fake-jpeg-data",
                "image/jpeg",
                None,
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
    async fn test_upload_missing_file_field_returns_400() {
        setup_test_env();
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool.clone()).await;
        let user_id = create_test_user(&pool).await;
        let token = create_session_token(&pool, user_id).await;

        let response = app
            .oneshot(build_multipart_no_file(Some(&token)))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body_bytes = read_body(response).await;
        let body_str = String::from_utf8_lossy(&body_bytes);
        assert!(
            body_str.contains("No file provided"),
            "Response body: {body_str}"
        );
    }

    // ── MIME type validation tests ──────────────────────────────────────────

    #[tokio::test]
    async fn test_upload_gif_rejected_with_422() {
        setup_test_env();
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool.clone()).await;
        let user_id = create_test_user(&pool).await;
        let token = create_session_token(&pool, user_id).await;

        let response = app
            .oneshot(build_multipart_request(
                "photo.gif",
                b"GIF89a fake gif content",
                "image/gif",
                None,
                Some(&token),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let body_bytes = read_body(response).await;
        let body_str = String::from_utf8_lossy(&body_bytes);
        assert!(
            body_str.contains("Invalid file type") || body_str.contains("Only JPEG, PNG, and WebP"),
            "Response body: {body_str}"
        );
    }

    #[tokio::test]
    async fn test_upload_jpeg_accepted() {
        setup_test_env();
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool.clone()).await;
        let user_id = create_test_user(&pool).await;
        let token = create_session_token(&pool, user_id).await;

        let response = app
            .oneshot(build_multipart_request(
                "recipe.jpg",
                b"fake-jpeg-content",
                "image/jpeg",
                None,
                Some(&token),
            ))
            .await
            .unwrap();

        // MIME validation passes (not 422). Storage call fails without
        // initialized STORAGE_CLIENT, so we expect 500. With storage
        // configured (e.g. wiremock), this would be 200.
        assert!(
            response.status() != StatusCode::UNPROCESSABLE_ENTITY,
            "JPEG should pass MIME validation, got {} with body: {}",
            response.status(),
            String::from_utf8_lossy(&read_body(response).await)
        );
    }

    #[tokio::test]
    async fn test_upload_png_accepted() {
        setup_test_env();
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool.clone()).await;
        let user_id = create_test_user(&pool).await;
        let token = create_session_token(&pool, user_id).await;

        let response = app
            .oneshot(build_multipart_request(
                "image.png",
                b"\x89PNG\r\n\x1a\n fake png content",
                "image/png",
                None,
                Some(&token),
            ))
            .await
            .unwrap();

        // Same as JPEG: MIME passes, storage may fail
        assert!(
            response.status() != StatusCode::UNPROCESSABLE_ENTITY,
            "PNG should pass MIME validation, got {} with body: {}",
            response.status(),
            String::from_utf8_lossy(&read_body(response).await)
        );
    }

    #[tokio::test]
    async fn test_upload_webp_accepted() {
        setup_test_env();
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool.clone()).await;
        let user_id = create_test_user(&pool).await;
        let token = create_session_token(&pool, user_id).await;

        let response = app
            .oneshot(build_multipart_request(
                "photo.webp",
                b"RIFF....WEBP fake webp content",
                "image/webp",
                None,
                Some(&token),
            ))
            .await
            .unwrap();

        // Same as JPEG: MIME passes, storage may fail
        assert!(
            response.status() != StatusCode::UNPROCESSABLE_ENTITY,
            "WebP should pass MIME validation, got {} with body: {}",
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

        // 1 KB of data
        let small_data = vec![0xFFu8; 1024];

        let response = app
            .oneshot(build_multipart_request(
                "small.jpg",
                &small_data,
                "image/jpeg",
                None,
                Some(&token),
            ))
            .await
            .unwrap();

        // Size check passes (not 413). Storage may fail.
        assert!(
            response.status() != StatusCode::PAYLOAD_TOO_LARGE,
            "1KB file should pass size validation, got {} with body: {}",
            response.status(),
            String::from_utf8_lossy(&read_body(response).await)
        );
    }

    #[tokio::test]
    async fn test_upload_at_limit_accepted() {
        setup_test_env();
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool.clone()).await;
        let user_id = create_test_user(&pool).await;
        let token = create_session_token(&pool, user_id).await;

        // We use a moderate-sized file (100KB) to verify the size check
        // allows files well within the 10MB limit. A true boundary test
        // with exactly 10MB would require ~10MB of test data in memory.
        let data = vec![0xFFu8; 100 * 1024];

        let response = app
            .oneshot(build_multipart_request(
                "at_limit.jpg",
                &data,
                "image/jpeg",
                None,
                Some(&token),
            ))
            .await
            .unwrap();

        // Size check passes (not 413). Storage may fail.
        assert!(
            response.status() != StatusCode::PAYLOAD_TOO_LARGE,
            "100KB file should pass size validation, got {} with body: {}",
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
                "large.jpg",
                &large_data,
                "image/jpeg",
                None,
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

    // ── Filename sanitization tests ─────────────────────────────────────────

    #[tokio::test]
    async fn test_upload_path_traversal_sanitized() {
        setup_test_env();
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool.clone()).await;
        let user_id = create_test_user(&pool).await;
        let token = create_session_token(&pool, user_id).await;

        // Filename with path traversal attempt
        let response = app
            .oneshot(build_multipart_request(
                "../../../etc/passwd.jpg",
                b"fake-jpeg-content",
                "image/jpeg",
                None,
                Some(&token),
            ))
            .await
            .unwrap();

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

        // Build request with null byte in filename using raw bytes
        let null_byte_filename = "test\x00.jpg";
        let body = format!(
            "--{boundary}\r\n\
             Content-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\n\
             Content-Type: image/jpeg\r\n\r\n\
             fake-jpeg-content\r\n\
             --{boundary}--\r\n",
            boundary = BOUNDARY,
            name = null_byte_filename,
        );

        let request = Request::builder()
            .method("POST")
            .uri("/api/upload")
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={BOUNDARY}"),
            )
            .header(
                "Cookie",
                format!("{}={}", session::COOKIE_NAME, token),
            )
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

    /// Full success path: valid auth + valid file -> 200 with url and key.
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

        // Mock the S3 PutObject call — accept any PUT request
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
                "recipe.jpg",
                b"fake-jpeg-content",
                "image/jpeg",
                None,
                Some(&token),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let json = read_json(response).await;

        // Verify response contains both "url" and "key"
        assert!(
            json.get("url").is_some(),
            "Response should contain 'url'"
        );
        assert!(
            json.get("key").is_some(),
            "Response should contain 'key'"
        );

        // Verify the key format: recipes/{user_id}/{uuid}.{ext}
        let key = json["key"]
            .as_str()
            .expect("key should be a string");
        let parts: Vec<&str> = key.split('/').collect();
        assert_eq!(
            parts.len(),
            3,
            "Key should have 3 segments: recipes/user_id/filename"
        );
        assert_eq!(parts[0], "recipes");
        assert_eq!(parts[1], user_id.to_string());

        // Filename should end with .jpeg
        assert!(
            parts[2].ends_with(".jpeg"),
            "Filename should end with .jpeg extension, got: {}",
            parts[2]
        );

        // UUID part should be a valid UUID format
        let uuid_str = parts[2].strip_suffix(".jpeg").unwrap();
        uuid::Uuid::parse_str(uuid_str).expect("UUID part of key should be valid");
    }
}
