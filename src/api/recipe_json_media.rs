//! Resolve images without fetching imported URLs on the server.
use crate::{
    db,
    storage::{MediaUrl, StorageClient},
    types::{ImportImageStatus, ImportedRecipeImage, RecipeEditorState, RecipeStep},
};
use dioxus::prelude::ServerFnError;
use std::time::Duration;
use uuid::Uuid;
pub fn external_url_allowed(value: &str) -> bool {
    url::Url::parse(value).ok().is_some_and(|url| url.scheme() == "https" && url.username().is_empty() && url.password().is_none()
        && matches!(url.host(), Some(url::Host::Domain(host)) if host.contains('.') && ![".localhost", ".local", ".internal"].iter().any(|suffix| host.trim_end_matches('.').ends_with(suffix))))
}
fn take_images(editor: &mut RecipeEditorState) -> Vec<ImportedRecipeImage> {
    fn take(
        images: &mut Vec<crate::types::RecipeImage>,
        path: &[usize],
        text: Option<&str>,
        output: &mut Vec<ImportedRecipeImage>,
    ) {
        for (position, image) in std::mem::take(images).into_iter().enumerate() {
            output.push(ImportedRecipeImage {
                id: Uuid::new_v4(),
                image,
                step_path: path.to_vec(),
                step_text: text.map(str::to_owned),
                position,
                status: ImportImageStatus::Unavailable,
            });
        }
    }
    fn steps(items: &mut [RecipeStep], path: &[usize], output: &mut Vec<ImportedRecipeImage>) {
        for (i, step) in items.iter_mut().enumerate() {
            let mut path = path.to_vec();
            path.push(i);
            take(&mut step.photos, &path, Some(&step.text), output);
            steps(&mut step.sub_steps, &path, output);
        }
    }
    let mut result = vec![];
    take(&mut editor.images, &[], None, &mut result);
    steps(&mut editor.instructions, &[], &mut result);
    result
}
pub fn external_urls(editor: &mut RecipeEditorState, storage: &StorageClient) -> Vec<String> {
    let mut urls = take_images(editor)
        .into_iter()
        .filter_map(|i| {
            (matches!(
                storage.classify_media_url(&i.image.url),
                Ok(MediaUrl::ExternalHttps)
            ) && external_url_allowed(&i.image.url))
            .then_some(i.image.url)
        })
        .collect::<Vec<_>>();
    urls.sort();
    urls.dedup();
    urls
}
#[allow(clippy::too_many_arguments)]
async fn check(
    pool: &sqlx::PgPool,
    storage: &StorageClient,
    user: Uuid,
    source: Option<Uuid>,
    candidates: Vec<ImportedRecipeImage>,
    available: &[String],
    draft: Option<Uuid>,
    deadline: tokio::time::Instant,
) -> Result<(Vec<ImportedRecipeImage>, Vec<ImportedRecipeImage>), ServerFnError> {
    if available.len() > crate::security::MAX_RECIPE_IMAGES
        || available
            .iter()
            .any(|u| u.len() > crate::security::MAX_IMAGE_METADATA_BYTES)
    {
        return Err(ServerFnError::new("Too many image checks"));
    }
    if candidates.is_empty() {
        return Ok((vec![], vec![]));
    }
    if tokio::time::Instant::now() >= deadline {
        return Ok((
            vec![],
            candidates
                .into_iter()
                .map(|mut image| {
                    image.status = ImportImageStatus::Unavailable;
                    image
                })
                .collect(),
        ));
    }
    let ids = candidates
        .iter()
        .filter_map(|i| storage.parse_media_url(&i.image.url).ok())
        .filter(|object| object.kind() == crate::storage::MediaObjectKind::RecipeFull)
        .map(|object| object.object_id())
        .collect::<Vec<_>>();
    let allowed = crate::security::run_with_database_budget(
        crate::security::DATABASE_EXECUTION_BUDGET
            .min(deadline.saturating_duration_since(tokio::time::Instant::now())),
        db::json_import_asset_access(pool, user, source, &ids, draft),
    )
    .await
    .map_err(|_| ServerFnError::new("Image permission check timed out"))?
    .map_err(|_| ServerFnError::new("Could not check image permissions"))?;
    let (mut checked, mut issues) = (vec![], vec![]);
    let mut cache = std::collections::HashMap::new();
    for mut candidate in candidates {
        let status = match storage.classify_media_url(&candidate.image.url) {
            Ok(MediaUrl::Internal(object))
                if object.kind() == crate::storage::MediaObjectKind::RecipeFull =>
            {
                if let Some(id) = allowed.get(&candidate.image.url) {
                    candidate.image.asset_id = Some(*id);
                    if let Some(status) = cache.get(&candidate.image.url) {
                        *status
                    } else {
                        let end =
                            deadline.min(tokio::time::Instant::now() + Duration::from_secs(3));
                        let status = if tokio::time::Instant::now() >= deadline {
                            Some(ImportImageStatus::Unavailable)
                        } else {
                            match tokio::time::timeout_at(
                                end,
                                storage.export_image_exists(object.key()),
                            )
                            .await
                            {
                                Ok(Ok(true)) => None,
                                Ok(Ok(false)) => Some(ImportImageStatus::Missing),
                                _ => Some(ImportImageStatus::Unavailable),
                            }
                        };
                        cache.insert(candidate.image.url.clone(), status);
                        status
                    }
                } else {
                    Some(ImportImageStatus::NotReusable)
                }
            }
            Ok(MediaUrl::ExternalHttps) if external_url_allowed(&candidate.image.url) => {
                candidate.image.asset_id = None;
                candidate.image.thumb_url = None;
                if available.contains(&candidate.image.url) {
                    None
                } else {
                    Some(ImportImageStatus::Unavailable)
                }
            }
            _ => Some(ImportImageStatus::UnsafeUrl),
        };
        if let Some(status) = status {
            candidate.status = status;
            issues.push(candidate);
        } else {
            checked.push(candidate);
        }
    }
    Ok((checked, issues))
}
#[allow(clippy::too_many_arguments)]
pub async fn prepare_collection_recipe(
    pool: &sqlx::PgPool,
    storage: &StorageClient,
    user: Uuid,
    source: Uuid,
    mut editor: RecipeEditorState,
    available: &[String],
    deadline: tokio::time::Instant,
) -> Result<db::PreparedJsonRecipe, ServerFnError> {
    let candidates = take_images(&mut editor);
    let relevant = candidates
        .iter()
        .filter(|image| available.contains(&image.image.url))
        .map(|image| image.image.url.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let (checked, issues) = check(
        pool,
        storage,
        user,
        Some(source),
        candidates,
        &relevant,
        None,
        deadline,
    )
    .await?;
    Ok(db::PreparedJsonRecipe {
        portable_id: format!("urn:noms:recipe:{source}"),
        payload: super::recipe::editor_to_draft_payload(editor),
        review: db::JsonImportReview {
            source: Some(source),
            images: issues,
        },
        checked,
    })
}

pub async fn import(
    pool: &sqlx::PgPool,
    storage: &StorageClient,
    user: Uuid,
    json: &str,
    available: &[String],
) -> Result<db::RecipeDraft, ServerFnError> {
    let (file, mut editor) = crate::recipe_json::parse_recipe(json).map_err(ServerFnError::new)?;
    let source = Some(crate::recipe_json::source_recipe_id(&file).map_err(ServerFnError::new)?);
    let candidates = take_images(&mut editor);
    let (checked, issues) = check(
        pool,
        storage,
        user,
        source,
        candidates,
        available,
        None,
        tokio::time::Instant::now() + Duration::from_secs(12),
    )
    .await?;
    crate::security::run_with_database_budget(
        crate::security::DATABASE_EXECUTION_BUDGET,
        db::create_json_import_draft(
            pool,
            user,
            super::recipe::editor_to_draft_payload(editor),
            db::JsonImportReview {
                source,
                images: issues,
            },
            checked,
        ),
    )
    .await
    .map_err(|_| ServerFnError::new("Import timed out. Try again."))?
    .map_err(|e| ServerFnError::new(format!("Could not save imported draft: {e}")))
}
#[allow(clippy::too_many_arguments)]
pub async fn resolve(
    pool: &sqlx::PgPool,
    storage: &StorageClient,
    user: Uuid,
    draft: Uuid,
    revision: i64,
    image: Uuid,
    omit: bool,
    available: &[String],
) -> Result<db::RecipeDraft, ServerFnError> {
    let original = crate::security::run_with_database_budget(
        crate::security::DATABASE_EXECUTION_BUDGET,
        db::get_recipe_draft(pool, draft, user),
    )
    .await
    .map_err(|_| ServerFnError::new("Draft load timed out"))?
    .map_err(|_| ServerFnError::new("Draft unavailable"))?;
    let review = db::json_import_review(&original)
        .map_err(|_| ServerFnError::new("Image review unavailable"))?;
    let candidate = review
        .images
        .into_iter()
        .find(|i| i.id == image)
        .ok_or_else(|| ServerFnError::new("Imported image not found"))?;
    let (checked, issues) = if omit {
        (vec![], vec![])
    } else {
        check(
            pool,
            storage,
            user,
            review.source,
            vec![candidate],
            available,
            Some(draft),
            tokio::time::Instant::now() + Duration::from_secs(12),
        )
        .await?
    };
    crate::security::run_with_database_budget(
        crate::security::DATABASE_EXECUTION_BUDGET,
        db::resolve_json_import_image(
            pool,
            user,
            draft,
            revision,
            image,
            checked.into_iter().next(),
            issues.first().map(|i| i.status),
        ),
    )
    .await
    .map_err(|_| ServerFnError::new("Image review timed out"))?
    .map_err(|_| {
        ServerFnError::new(
            "The draft changed or an image limit was reached. Save or reload before retrying.",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};
    async fn fixture() -> (
        pgtemp::PgTempDB,
        sqlx::PgPool,
        MockServer,
        StorageClient,
        crate::types::Recipe,
        Uuid,
    ) {
        let (db, pool) = crate::test_utils::setup_test_db().await;
        let mock = MockServer::start().await;
        let storage = StorageClient::for_test_with_endpoint_and_public_url(
            &mock.uri(),
            "https://saved.example/media",
        );
        let (recipe, owner) = super::super::recipe_archive::tests::source(&pool, &storage).await;
        (db, pool, mock, storage, recipe, owner)
    }
    async fn reader(pool: &sqlx::PgPool) -> Uuid {
        let suffix = crate::test_utils::uid();
        db::insert_user(
            pool,
            &format!("reader_{suffix}"),
            "Reader",
            &format!("reader_{suffix}@example.test"),
            None,
        )
        .await
        .unwrap()
        .id
    }
    fn encode(recipe: &crate::types::Recipe) -> String {
        crate::recipe_json::export_recipe(recipe, vec!["import".into()]).unwrap()
    }
    #[tokio::test]
    async fn references_reuse_accessible_current_media_and_ignore_forged_foreign_asset_ids() {
        let (_db, pool, mock, storage, recipe, owner) = fixture().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock)
            .await;
        let stranger = reader(&pool).await;
        let blocked = import(&pool, &storage, stranger, &encode(&recipe), &[])
            .await
            .unwrap();
        assert!(blocked.decoded_snapshot().unwrap().images.is_empty());
        assert_eq!(
            db::json_import_review(&blocked)
                .unwrap()
                .images
                .iter()
                .filter(|i| i.status == ImportImageStatus::NotReusable)
                .count(),
            3
        );
        assert!(
            mock.received_requests().await.unwrap().is_empty(),
            "No storage probing before permission checks"
        );
        for (actor, visibility) in [
            (owner, "private"),
            (stranger, "public"),
            (stranger, "unlisted"),
        ] {
            db::update_recipe_visibility(&pool, recipe.id, owner, visibility)
                .await
                .unwrap();
            let draft = import(
                &pool,
                &storage,
                actor,
                &encode(&recipe),
                &["https://external.example/photo.webp".into()],
            )
            .await
            .unwrap();
            let content = draft.decoded_snapshot().unwrap();
            assert_eq!(content.images.len(), 2);
            assert_eq!(content.images[0].asset_id, recipe.images[0].asset_id);
            assert_eq!(content.images[1].asset_id, None);
            assert_eq!(
                content.instructions[0].sub_steps[0].photos,
                recipe.instructions[0].sub_steps[0].photos
            );
            assert!(db::json_import_review(&draft).unwrap().images.is_empty());
            let refs: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM recipe_draft_media_refs WHERE recipe_draft_id = $1",
            )
            .bind(draft.id)
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(refs, 2);
        }
        let mut foreign = recipe.clone();
        foreign.images[1].asset_id = recipe.images[0].asset_id;
        let draft = import(
            &pool,
            &storage,
            stranger,
            &encode(&foreign),
            &[foreign.images[1].url.clone()],
        )
        .await
        .unwrap();
        assert!(draft.decoded_snapshot().unwrap().images[1]
            .asset_id
            .is_none());
        assert_eq!(
            mock.received_requests().await.unwrap().len(),
            8,
            "Only unique managed HEAD requests, never external fetches"
        );
    }
    #[tokio::test]
    async fn missing_image_review_survives_autosave_reload_retry_omit_and_revision_guards() {
        let (_db, pool, mock, storage, recipe, owner) = fixture().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock)
            .await;
        let draft = import(&pool, &storage, owner, &encode(&recipe), &[])
            .await
            .unwrap();
        let review = db::json_import_review(&draft).unwrap();
        assert_eq!(review.images.len(), 4);
        assert_eq!(
            review
                .images
                .iter()
                .filter(|i| i.status == ImportImageStatus::Missing)
                .count(),
            3
        );
        assert_eq!(review.images[1].status, ImportImageStatus::Unavailable);
        let mut payload = draft.decoded_snapshot().unwrap();
        payload.content.title = "Edited after importing".into();
        let saved = db::update_recipe_draft(&pool, draft.id, owner, draft.revision, &payload)
            .await
            .unwrap();
        let loaded = db::get_recipe_draft(&pool, draft.id, owner).await.unwrap();
        assert_eq!(
            db::json_import_review(&loaded).unwrap().images,
            review.images
        );
        mock.reset().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock)
            .await;
        let restored = resolve(
            &pool,
            &storage,
            owner,
            draft.id,
            saved.revision,
            review.images[0].id,
            false,
            &[],
        )
        .await
        .unwrap();
        assert_eq!(
            restored.decoded_snapshot().unwrap().title,
            "Edited after importing"
        );
        assert_eq!(restored.decoded_snapshot().unwrap().images.len(), 1);
        assert_eq!(db::json_import_review(&restored).unwrap().images.len(), 3);
        assert!(resolve(
            &pool,
            &storage,
            owner,
            draft.id,
            saved.revision,
            review.images[1].id,
            true,
            &[]
        )
        .await
        .is_err());
        let stranger = reader(&pool).await;
        assert!(resolve(
            &pool,
            &storage,
            stranger,
            draft.id,
            restored.revision,
            review.images[1].id,
            true,
            &[]
        )
        .await
        .is_err());
        let omitted = resolve(
            &pool,
            &storage,
            owner,
            draft.id,
            restored.revision,
            review.images[1].id,
            true,
            &[],
        )
        .await
        .unwrap();
        assert_eq!(db::json_import_review(&omitted).unwrap().images.len(), 2);
        let mut payload = omitted.decoded_snapshot().unwrap();
        payload.instructions[0].text = "Changed destination".into();
        let saved = db::update_recipe_draft(&pool, draft.id, owner, omitted.revision, &payload)
            .await
            .unwrap();
        let moved = resolve(
            &pool,
            &storage,
            owner,
            draft.id,
            saved.revision,
            review.images[2].id,
            false,
            &[],
        )
        .await
        .unwrap();
        assert!(db::json_import_review(&moved)
            .unwrap()
            .images
            .iter()
            .any(|i| i.status == ImportImageStatus::PlacementChanged));
    }
    #[tokio::test]
    async fn transient_failures_and_revoked_permissions_preserve_text_without_attaching_media() {
        let (_db, pool, mock, storage, recipe, owner) = fixture().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&mock)
            .await;
        let draft = import(&pool, &storage, owner, &encode(&recipe), &[])
            .await
            .unwrap();
        assert_eq!(draft.decoded_snapshot().unwrap().title, recipe.title);
        assert!(db::json_import_review(&draft)
            .unwrap()
            .images
            .iter()
            .all(|i| i.status == ImportImageStatus::Unavailable));
        let stranger = reader(&pool).await;
        let (_, mut editor) = crate::recipe_json::parse_recipe(&encode(&recipe)).unwrap();
        let mut checked = take_images(&mut editor);
        checked.retain(|i| i.image.asset_id.is_some());
        let draft = db::create_json_import_draft(
            &pool,
            stranger,
            super::super::recipe::editor_to_draft_payload(editor),
            db::JsonImportReview {
                source: Some(recipe.id),
                images: vec![],
            },
            checked,
        )
        .await
        .unwrap();
        assert!(draft.decoded_snapshot().unwrap().images.is_empty());
        assert!(db::json_import_review(&draft)
            .unwrap()
            .images
            .iter()
            .all(|i| i.status == ImportImageStatus::NotReusable));
    }
    #[tokio::test]
    async fn a_draft_can_reuse_its_retained_image_after_source_permission_is_revoked() {
        let (_db, pool, mock, storage, recipe, owner) = fixture().await;
        db::update_recipe_visibility(&pool, recipe.id, owner, "public")
            .await
            .unwrap();
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock)
            .await;
        let user = reader(&pool).await;
        let draft = import(&pool, &storage, user, &encode(&recipe), &[])
            .await
            .unwrap();
        let review = db::json_import_review(&draft).unwrap();
        mock.reset().await;
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock)
            .await;
        let restored = resolve(
            &pool,
            &storage,
            user,
            draft.id,
            draft.revision,
            review.images[0].id,
            false,
            &[],
        )
        .await
        .unwrap();
        db::update_recipe_visibility(&pool, recipe.id, owner, "private")
            .await
            .unwrap();
        let repeated = resolve(
            &pool,
            &storage,
            user,
            draft.id,
            restored.revision,
            review.images[2].id,
            false,
            &[],
        )
        .await
        .unwrap();
        assert_eq!(
            repeated.decoded_snapshot().unwrap().instructions[0]
                .photos
                .len(),
            1
        );
    }

    #[test]
    fn external_checks_exclude_active_schemes_and_obvious_local_hosts() {
        for url in [
            "http://example.test/photo",
            "https://127.0.0.1/photo",
            "https://[::1]/photo",
            "https://localhost/photo",
            "https://host.local/photo",
            "https://host.internal/photo",
            "https://u:p@example.test/photo",
            "data:image/png,hi",
            "javascript:alert(1)",
        ] {
            assert!(!external_url_allowed(url), "{url}");
        }
        assert!(external_url_allowed(
            "https://images.example.test/photo.webp?signature=abc"
        ));
    }
}
