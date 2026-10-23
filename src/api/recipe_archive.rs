//! Optional image files alongside reference-preserving recipe JSON.
use crate::{
    auth::context::AuthUser,
    storage::{MediaObjectKind, MediaUrl, StorageClient},
    types::{Recipe, RecipeImage, RecipeStep},
};
use axum::{
    extract::{Extension, Path},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use std::{
    collections::HashSet,
    io::{Cursor, Write},
    time::Duration,
};
use uuid::Uuid;

const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;
const MAX_ARCHIVE_BYTES: usize = 32 * 1024 * 1024;
type ImageFiles = Vec<(String, String)>;
type Failure = (StatusCode, &'static str);

pub async fn download(
    Path(id): Path<Uuid>,
    Extension(pool): Extension<sqlx::PgPool>,
    Extension(storage): Extension<StorageClient>,
    headers: HeaderMap,
    viewer: Option<Extension<AuthUser>>,
) -> Response {
    let actor = viewer.map(|Extension(user)| user.user_id);
    let response = match crate::security::enforce_resolved_request_budget(
        actor,
        headers
            .get(&*crate::middleware::rate_limit::RESOLVED_CLIENT_IP)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse().ok()),
        crate::security::RequestClass::Graph,
    ) {
        Err(_) => (
            StatusCode::TOO_MANY_REQUESTS,
            "Too many export requests. Try again shortly.",
        )
            .into_response(),
        Ok(()) => {
            match crate::security::run_with_expensive_read_work_budget(tokio::time::timeout(
                Duration::from_secs(60),
                prepare_archive(&pool, &storage, id, actor),
            ))
            .await
            {
                Ok(Ok(Ok(bytes))) => (
                    [
                        (header::CONTENT_TYPE, "application/zip".to_string()),
                        (
                            header::CONTENT_DISPOSITION,
                            format!("attachment; filename=\"noms-recipe-{id}.zip\""),
                        ),
                    ],
                    bytes,
                )
                    .into_response(),
                Ok(Ok(Err(error))) => error.into_response(),
                Ok(Err(_)) => (
                    StatusCode::GATEWAY_TIMEOUT,
                    "Image download timed out. Try JSON only.",
                )
                    .into_response(),
                Err(_) => (
                    StatusCode::TOO_MANY_REQUESTS,
                    "Exports are busy. Try again shortly.",
                )
                    .into_response(),
            }
        }
    };
    let mut response = response;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "private, no-store".parse().unwrap());
    response
}

async fn prepare_archive(
    pool: &sqlx::PgPool,
    storage: &StorageClient,
    id: Uuid,
    viewer: Option<Uuid>,
) -> Result<Vec<u8>, Failure> {
    let (recipe, json) = crate::security::run_with_database_budget(
        crate::security::DATABASE_EXECUTION_BUDGET,
        super::recipe::read_recipe_export(pool, id, viewer),
    )
    .await
    .map_err(|_| (StatusCode::GATEWAY_TIMEOUT, "Recipe export timed out."))?
    .map_err(|error| {
        if error.to_string().contains("Recipe not found") {
            (StatusCode::NOT_FOUND, "Recipe not found")
        } else {
            (StatusCode::INTERNAL_SERVER_ERROR, "Could not read recipe.")
        }
    })?;
    let (images, external_count) = image_files(storage, &recipe).map_err(|_| {
        (
            StatusCode::BAD_GATEWAY,
            "Could not prepare saved images. Try JSON only.",
        )
    })?;
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let mut total = 0;
    write_file(&mut zip, &mut total, "recipe.json", json.as_bytes())?;
    let readme = format!(
        "Import recipe.json into Noms. Accessible image links are reused; add these files manually if a linked image cannot be restored.\n\
         These are saved full-size image files; editor crops and adjustments are not baked in.\n\
         Recipe and step filenames indicate their original placement. Repeated files are included once.\n\
         {external_count} externally linked images were not downloaded. Only images stored in Noms are included.\n"
    );
    write_file(&mut zip, &mut total, "README.txt", readme.as_bytes())?;
    for (name, key) in images {
        let limit = MAX_IMAGE_BYTES.min(MAX_ARCHIVE_BYTES.saturating_sub(total));
        let bytes = storage
            .read_export_image(&key, limit)
            .await
            .map_err(|error| match error {
                crate::storage::ExportImageError::TooLarge => (
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "Saved images exceed the download limit. Try JSON only.",
                ),
                crate::storage::ExportImageError::Unavailable => (
                    StatusCode::BAD_GATEWAY,
                    "A saved image is unavailable. Try JSON only.",
                ),
            })?;
        if !super::upload::validate_webp_magic(&bytes) {
            return Err((
                StatusCode::BAD_GATEWAY,
                "A saved image is not a supported image file. Try JSON only.",
            ));
        }
        write_file(&mut zip, &mut total, &name, &bytes)?;
    }
    let bytes = zip
        .finish()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Could not finish ZIP."))?
        .into_inner();
    if bytes.len() > MAX_ARCHIVE_BYTES {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            "Image ZIP exceeds 32 MiB. Try JSON only.",
        ));
    }
    Ok(bytes)
}

fn write_file(
    zip: &mut zip::ZipWriter<Cursor<Vec<u8>>>,
    total: &mut usize,
    name: &str,
    bytes: &[u8],
) -> Result<(), Failure> {
    // Reserve space for ZIP headers and the central directory as well as content.
    if bytes.len().saturating_add(*total).saturating_add(64 * 1024) > MAX_ARCHIVE_BYTES {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            "Image ZIP exceeds 32 MiB. Try JSON only.",
        ));
    }
    zip.start_file(
        name,
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
    )
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Could not prepare ZIP."))?;
    zip.write_all(bytes)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Could not prepare ZIP."))?;
    *total += bytes.len();
    Ok(())
}

fn image_files(storage: &StorageClient, recipe: &Recipe) -> Result<(ImageFiles, usize), String> {
    fn add(
        storage: &StorageClient,
        image: &RecipeImage,
        name: String,
        seen: &mut HashSet<String>,
        files: &mut Vec<(String, String)>,
        external: &mut usize,
    ) -> Result<(), String> {
        match storage.classify_media_url(&image.url)? {
            MediaUrl::ExternalHttps => *external += 1,
            MediaUrl::Internal(object) => {
                if object.kind() != MediaObjectKind::RecipeFull || image.asset_id.is_none() {
                    return Err("Invalid saved recipe image".into());
                }
                if seen.insert(object.key().to_string()) {
                    files.push((name, object.key().to_string()));
                }
            }
        }
        Ok(())
    }
    fn steps(
        storage: &StorageClient,
        items: &[RecipeStep],
        prefix: &str,
        seen: &mut HashSet<String>,
        files: &mut Vec<(String, String)>,
        external: &mut usize,
    ) -> Result<(), String> {
        for (i, step) in items.iter().enumerate() {
            let path = format!("{prefix}{}", i + 1);
            for (j, image) in step.photos.iter().enumerate() {
                add(
                    storage,
                    image,
                    format!("images/step-{path}-photo-{:03}.webp", j + 1),
                    seen,
                    files,
                    external,
                )?;
            }
            steps(
                storage,
                &step.sub_steps,
                &format!("{path}-"),
                seen,
                files,
                external,
            )?;
        }
        Ok(())
    }
    let (mut seen, mut files, mut external) = (HashSet::new(), Vec::new(), 0);
    for (i, image) in recipe.images.iter().enumerate() {
        add(
            storage,
            image,
            format!("images/recipe-{:03}.webp", i + 1),
            &mut seen,
            &mut files,
            &mut external,
        )?;
    }
    steps(
        storage,
        &recipe.instructions,
        "",
        &mut seen,
        &mut files,
        &mut external,
    )?;
    Ok((files, external))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        db::{
            self,
            diff::{RecipeDraftPayload, RecipeSnapshot},
            MediaPurpose, NewMediaAsset,
        },
        types::{RecipeImageEdit, RecipeIngredient},
    };
    use axum::{
        body::{to_bytes, Body},
        http::Request,
        routing::get,
        Router,
    };
    use tower::ServiceExt;
    use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

    pub(crate) async fn source(pool: &sqlx::PgPool, storage: &StorageClient) -> (Recipe, Uuid) {
        let suffix = crate::test_utils::uid();
        let owner = db::insert_user(
            pool,
            &format!("archive_{suffix}"),
            "Archive test",
            &format!("{suffix}@example.test"),
            None,
        )
        .await
        .unwrap();
        let mut images = Vec::new();
        for _ in 0..2 {
            let id = Uuid::new_v4();
            let full_key = format!("recipes/{}/{id}_full.webp", owner.id);
            let thumb_key = format!("recipes/{}/{id}_thumb.webp", owner.id);
            let image = RecipeImage::managed(
                id,
                storage.object_url(&full_key),
                Some(storage.object_url(&thumb_key)),
                RecipeImageEdit::default_rect_16x9(),
            );
            db::register_media_asset(
                pool,
                &NewMediaAsset {
                    id,
                    uploader_id: owner.id,
                    purpose: MediaPurpose::Recipe,
                    full_url: image.url.clone(),
                    thumbnail_url: image.thumb_url.clone().unwrap(),
                    original_url: None,
                    full_key,
                    thumbnail_key: thumb_key,
                    original_key: None,
                    capability_digest: Some(id.as_bytes().repeat(2)),
                    expires_at: Some(chrono::Utc::now() + chrono::Duration::minutes(15)),
                },
            )
            .await
            .unwrap();
            images.push(image);
        }
        let recipe = db::create_published_recipe(
            pool,
            owner.id,
            &RecipeDraftPayload {
                initial_visibility: Some("private".into()),
                content: RecipeSnapshot {
                    title: "Archive fixture".into(),
                    description: None,
                    commentary: None,
                    prep_time_minutes: None,
                    cook_time_minutes: None,
                    servings: None,
                    ingredients: vec![RecipeIngredient {
                        vocabulary_id: None,
                        amount: "1".into(),
                        unit: "cup".into(),
                        name: "beans".into(),
                    }],
                    images: vec![
                        images[0].clone(),
                        RecipeImage::new(
                            "https://external.example/photo.webp".into(),
                            None,
                            RecipeImageEdit::default_rect_16x9(),
                        ),
                    ],
                    instructions: vec![RecipeStep {
                        text: "Cook".into(),
                        photos: vec![images[0].clone()],
                        sub_steps: vec![RecipeStep {
                            text: "Stir".into(),
                            photos: vec![images[1].clone()],
                            sub_steps: vec![],
                        }],
                    }],
                    equipment: vec![],
                    tags: vec![],
                },
            },
            None,
        )
        .await
        .unwrap();
        (recipe, owner.id)
    }

    fn router(pool: sqlx::PgPool, storage: StorageClient, actor: Uuid) -> Router {
        Router::new()
            .route("/download/{id}", get(download))
            .layer(Extension(pool))
            .layer(Extension(storage))
            .layer(Extension(AuthUser { user_id: actor }))
    }

    #[tokio::test]
    async fn image_zip_checks_access_and_contains_canonical_json_and_unique_saved_files() {
        let (_db, pool) = crate::test_utils::setup_test_db().await;
        let mock = MockServer::start().await;
        let bytes = b"RIFF0000WEBPsynthetic";
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(bytes))
            .mount(&mock)
            .await;
        let storage = StorageClient::for_test_with_endpoint_and_public_url(
            &mock.uri(),
            "https://saved.example/media",
        );
        let (recipe, owner) = source(&pool, &storage).await;
        let denied = router(pool.clone(), storage.clone(), Uuid::new_v4())
            .oneshot(
                Request::builder()
                    .uri(format!("/download/{}", recipe.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(denied.status(), StatusCode::NOT_FOUND);
        assert!(mock.received_requests().await.unwrap().is_empty());
        for visibility in ["private", "public", "unlisted"] {
            db::update_recipe_visibility(&pool, recipe.id, owner, visibility)
                .await
                .unwrap();
            let actor = if visibility == "private" {
                owner
            } else {
                Uuid::new_v4()
            };
            let response = router(pool.clone(), storage.clone(), actor)
                .oneshot(
                    Request::builder()
                        .uri(format!("/download/{}", recipe.id))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.headers()[header::CONTENT_TYPE], "application/zip");
            assert_eq!(
                response.headers()[header::CACHE_CONTROL],
                "private, no-store"
            );
            let body = to_bytes(response.into_body(), MAX_ARCHIVE_BYTES)
                .await
                .unwrap();
            let mut zip = zip::ZipArchive::new(Cursor::new(body)).unwrap();
            let mut names = zip
                .file_names()
                .map(|name| name.unwrap().into_owned())
                .collect::<Vec<_>>();
            names.sort();
            assert_eq!(
                names,
                [
                    "README.txt",
                    "images/recipe-001.webp",
                    "images/step-1-1-photo-001.webp",
                    "recipe.json"
                ]
            );
            let mut json = String::new();
            std::io::Read::read_to_string(&mut zip.by_name("recipe.json").unwrap(), &mut json)
                .unwrap();
            assert!(json.contains("saved.example"));
            assert!(json.contains("external.example"));
            let (_, editor) = crate::recipe_json::parse_recipe(&json).unwrap();
            assert_eq!(editor.images.len(), 2);
            assert_eq!(editor.instructions[0].sub_steps[0].photos.len(), 1);
            let mut image = Vec::new();
            std::io::Read::read_to_end(
                &mut zip.by_name("images/recipe-001.webp").unwrap(),
                &mut image,
            )
            .unwrap();
            assert_eq!(image, bytes);
        }
        let guest = Router::new()
            .route("/download/{id}", get(download))
            .layer(Extension(pool.clone()))
            .layer(Extension(storage.clone()));
        let response = guest
            .oneshot(
                Request::builder()
                    .uri(format!("/download/{}", recipe.id))
                    .header(
                        &*crate::middleware::rate_limit::RESOLVED_CLIENT_IP,
                        "198.51.100.41",
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "anonymous viewers can download unlisted recipes"
        );
        assert_eq!(
            mock.received_requests().await.unwrap().len(),
            8,
            "download each unique saved image once, never external URLs"
        );
    }

    #[tokio::test]
    async fn image_zip_fails_clearly_for_missing_images_and_bounds_object_reads() {
        let (_db, pool) = crate::test_utils::setup_test_db().await;
        let mock = MockServer::start().await;
        let storage = StorageClient::for_test_with_endpoint_and_public_url(
            &mock.uri(),
            "https://saved.example/media",
        );
        let (recipe, owner) = source(&pool, &storage).await;
        let response = router(pool.clone(), storage.clone(), owner)
            .oneshot(
                Request::builder()
                    .uri(format!("/download/{}", recipe.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        assert!(!response.headers().contains_key(header::CONTENT_DISPOSITION));
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"RIFF0000WEBP".as_slice()))
            .mount(&mock)
            .await;
        assert!(storage.read_export_image("test.webp", 4).await.is_err());
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let mut total = MAX_ARCHIVE_BYTES;
        assert_eq!(
            write_file(&mut zip, &mut total, "test.webp", b"test")
                .unwrap_err()
                .0,
            StatusCode::PAYLOAD_TOO_LARGE
        );
        // Image retrieval failure cannot prevent a plain JSON export.
        assert!(
            super::super::recipe::read_recipe_export(&pool, recipe.id, Some(owner))
                .await
                .is_ok()
        );
    }
}
