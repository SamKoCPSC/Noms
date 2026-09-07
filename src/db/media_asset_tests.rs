use super::*;
use chrono::{Duration, Utc};
use std::time::Duration as StdDuration;

async fn owner(pool: &PgPool, prefix: &str) -> User {
    let suffix = crate::test_utils::uid();
    insert_user(
        pool,
        &format!("{prefix}_{suffix}"),
        "Media Asset Owner",
        &format!("{prefix}_{suffix}@example.test"),
        None,
    )
    .await
    .unwrap()
}

fn asset(uploader_id: Uuid, purpose: MediaPurpose, marker: u8) -> NewMediaAsset {
    let id = Uuid::new_v4();
    let namespace = match purpose {
        MediaPurpose::Recipe => "recipes",
        MediaPurpose::CollectionCover => "collections",
        MediaPurpose::Avatar => "avatars",
    };
    let full_key = format!("{namespace}/{uploader_id}/{id}_full.webp");
    let thumbnail_key = format!("{namespace}/{uploader_id}/{id}_thumb.webp");
    let original_key = (purpose == MediaPurpose::Avatar)
        .then(|| format!("{namespace}/{uploader_id}/{id}_original"));
    NewMediaAsset {
        id,
        uploader_id,
        purpose,
        full_url: format!("https://media.example.test/{full_key}"),
        thumbnail_url: format!("https://media.example.test/{thumbnail_key}"),
        original_url: original_key
            .as_ref()
            .map(|key| format!("https://media.example.test/{key}")),
        full_key,
        thumbnail_key,
        original_key,
        capability_digest: (purpose != MediaPurpose::Avatar).then(|| vec![marker; 32]),
        expires_at: (purpose != MediaPurpose::Avatar).then(|| Utc::now() + Duration::minutes(15)),
    }
}

fn managed_image(asset: &NewMediaAsset) -> RecipeImage {
    RecipeImage::managed(
        asset.id,
        asset.full_url.clone(),
        Some(asset.thumbnail_url.clone()),
        crate::types::RecipeImageEdit::default_rect_16x9(),
    )
}

fn snapshot(title: &str, images: Vec<RecipeImage>, steps: Vec<RecipeStep>) -> diff::RecipeSnapshot {
    let instructions = if steps.is_empty() {
        vec![RecipeStep {
            text: "Cook".to_string(),
            sub_steps: Vec::new(),
            photos: Vec::new(),
        }]
    } else {
        steps
    };
    diff::RecipeSnapshot {
        title: title.to_string(),
        description: None,
        commentary: None,
        prep_time_minutes: None,
        cook_time_minutes: None,
        servings: None,
        ingredients: vec![RecipeIngredient {
            amount: "1".to_string(),
            unit: "cup".to_string(),
            name: "beans".to_string(),
        }],
        instructions,
        equipment: Vec::new(),
        images,
        tags: Vec::new(),
    }
}

#[test]
fn schema_uses_assets_and_explicit_holder_references_only() {
    let schema = include_str!("../../migrations/schema.sql");
    for table in [
        "media_assets",
        "recipe_media_refs",
        "recipe_version_media_refs",
        "recipe_draft_media_refs",
        "collection_media_refs",
        "avatar_media_refs",
    ] {
        assert!(schema.contains(&format!("CREATE TABLE IF NOT EXISTS {table}")));
    }
    assert!(schema.contains("uploader_id UUID REFERENCES users(id) ON DELETE SET NULL"));
    assert!(!schema.contains("CREATE TABLE IF NOT EXISTS media_upload_families"));
    assert!(!schema.contains("CREATE TABLE IF NOT EXISTS account_media_cleanup_tombstones"));
    assert!(
        !schema.contains("jsonb_path_ops"),
        "media cleanup must not rely on JSON indexes"
    );
}

#[tokio::test]
async fn uncommitted_asset_is_not_swept_before_expiry_and_discard_is_one_use() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let user = owner(&pool, "asset_grace").await;
    let upload = asset(user.id, MediaPurpose::Recipe, 11);
    register_media_asset(&pool, &upload).await.unwrap();

    assert_eq!(
        claim_media_cleanup(&pool, upload.id).await.unwrap(),
        MediaDiscardDecision::Retained,
        "ordinary cleanup cannot claim an unexpired upload"
    );
    let digest = upload.capability_digest.as_deref().unwrap();
    assert!(matches!(
        claim_media_discard(&pool, user.id, digest).await.unwrap(),
        MediaDiscardDecision::Claimed(_)
    ));
    assert_eq!(
        claim_media_discard(&pool, user.id, digest).await.unwrap(),
        MediaDiscardDecision::Denied
    );
}

#[tokio::test]
async fn expired_uncommitted_asset_cannot_be_attached_or_discarded_by_capability() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let user = owner(&pool, "asset_expired").await;
    let mut upload = asset(user.id, MediaPurpose::Recipe, 12);
    upload.expires_at = Some(Utc::now() - Duration::minutes(1));
    register_media_asset(&pool, &upload).await.unwrap();

    let result = create_published_recipe(
        &pool,
        user.id,
        &diff::RecipeDraftPayload {
            initial_visibility: Some("private".to_string()),
            content: snapshot("Expired", vec![managed_image(&upload)], Vec::new()),
        },
        None,
    )
    .await;
    assert!(
        result.is_err(),
        "an expired upload must not become retained"
    );
    assert_eq!(
        claim_media_discard(&pool, user.id, upload.capability_digest.as_deref().unwrap(),)
            .await
            .unwrap(),
        MediaDiscardDecision::Denied,
        "the upload capability expires with the attachment grace period"
    );
    assert!(matches!(
        claim_media_cleanup(&pool, upload.id).await.unwrap(),
        MediaDiscardDecision::Claimed(_)
    ));
}

#[tokio::test]
async fn managed_media_urls_must_match_the_asset_identity() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let user = owner(&pool, "asset_url_binding").await;
    let upload = asset(user.id, MediaPurpose::Recipe, 13);
    register_media_asset(&pool, &upload).await.unwrap();
    let mut image = managed_image(&upload);
    image.url = "https://attacker.example/not-the-registered-object.webp".to_string();

    let result = create_published_recipe(
        &pool,
        user.id,
        &diff::RecipeDraftPayload {
            initial_visibility: Some("private".to_string()),
            content: snapshot("Mismatched URL", vec![image], Vec::new()),
        },
        None,
    )
    .await;

    assert!(result.is_err());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM recipe_media_refs WHERE asset_id = $1")
            .bind(upload.id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn attachment_and_cleanup_serialize_on_the_asset_row() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let user = owner(&pool, "asset_race").await;
    let upload = asset(user.id, MediaPurpose::Recipe, 14);
    register_media_asset(&pool, &upload).await.unwrap();
    let recipe = insert_recipe(
        &pool,
        user.id,
        "Race holder",
        None,
        None,
        None,
        None,
        None,
        &[],
        &[],
        &[],
        &[],
        "private",
    )
    .await
    .unwrap();
    let image = managed_image(&upload);
    let mut transaction = pool.begin().await.unwrap();
    sync_recipe_media_references(
        &mut transaction,
        MediaHolder::Recipe(recipe.id),
        user.id,
        MediaPurpose::Recipe,
        std::slice::from_ref(&image),
        &[],
        MediaAttachAuthority::UploaderOnly,
    )
    .await
    .unwrap();

    let cleanup_pool = pool.clone();
    let mut cleanup =
        tokio::spawn(async move { claim_media_cleanup(&cleanup_pool, upload.id).await });
    assert!(
        tokio::time::timeout(StdDuration::from_millis(50), &mut cleanup)
            .await
            .is_err(),
        "cleanup must wait for the attachment transaction's row lock"
    );
    transaction.commit().await.unwrap();
    assert_eq!(
        cleanup.await.unwrap().unwrap(),
        MediaDiscardDecision::Retained
    );
}

#[tokio::test]
async fn external_recipe_images_create_no_managed_asset_references() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let user = owner(&pool, "asset_external").await;
    let external = RecipeImage::new(
        "https://images.example.test/recipe.webp".to_string(),
        None,
        crate::types::RecipeImageEdit::default_rect_16x9(),
    );
    let recipe = create_published_recipe(
        &pool,
        user.id,
        &diff::RecipeDraftPayload {
            initial_visibility: Some("private".to_string()),
            content: snapshot("External", vec![external], Vec::new()),
        },
        None,
    )
    .await
    .unwrap();

    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM recipe_media_refs WHERE recipe_id = $1",
        )
        .bind(recipe.id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
}

#[tokio::test]
async fn recipe_top_level_and_nested_media_have_explicit_holder_references() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let user = owner(&pool, "asset_recipe").await;
    let top = asset(user.id, MediaPurpose::Recipe, 21);
    let nested = asset(user.id, MediaPurpose::Recipe, 22);
    register_media_asset(&pool, &top).await.unwrap();
    register_media_asset(&pool, &nested).await.unwrap();

    let recipe = create_published_recipe(
        &pool,
        user.id,
        &diff::RecipeDraftPayload {
            initial_visibility: Some("private".to_string()),
            content: snapshot(
                "Referenced recipe",
                vec![managed_image(&top)],
                vec![RecipeStep {
                    text: "Cook".to_string(),
                    sub_steps: vec![RecipeStep {
                        text: "Serve".to_string(),
                        sub_steps: Vec::new(),
                        photos: vec![managed_image(&nested)],
                    }],
                    photos: Vec::new(),
                }],
            ),
        },
        None,
    )
    .await
    .unwrap();

    let recipe_refs: Vec<Uuid> = sqlx::query_scalar(
        "SELECT asset_id FROM recipe_media_refs WHERE recipe_id = $1 ORDER BY asset_id",
    )
    .bind(recipe.id)
    .fetch_all(&pool)
    .await
    .unwrap();
    let version_refs: Vec<Uuid> = sqlx::query_scalar(
        "SELECT ref.asset_id FROM recipe_version_media_refs ref \
         JOIN recipe_versions version ON version.id = ref.recipe_version_id \
         WHERE version.recipe_id = $1 ORDER BY ref.asset_id",
    )
    .bind(recipe.id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(recipe_refs.len(), 2);
    assert_eq!(version_refs, recipe_refs);
    assert_eq!(
        claim_media_cleanup(&pool, top.id).await.unwrap(),
        MediaDiscardDecision::Retained
    );

    delete_recipe(&pool, recipe.id, user.id).await.unwrap();
    assert!(matches!(
        claim_media_cleanup(&pool, top.id).await.unwrap(),
        MediaDiscardDecision::Claimed(_)
    ));
}

#[tokio::test]
async fn fork_references_survive_source_account_and_remain_scoped_to_the_fork() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let source_owner = owner(&pool, "asset_source").await;
    let fork_owner = owner(&pool, "asset_fork").await;
    let upload = asset(source_owner.id, MediaPurpose::Recipe, 31);
    register_media_asset(&pool, &upload).await.unwrap();
    let source = create_published_recipe(
        &pool,
        source_owner.id,
        &diff::RecipeDraftPayload {
            initial_visibility: Some("public".to_string()),
            content: snapshot("Source", vec![managed_image(&upload)], Vec::new()),
        },
        None,
    )
    .await
    .unwrap();

    let unrelated = create_published_recipe(
        &pool,
        fork_owner.id,
        &diff::RecipeDraftPayload {
            initial_visibility: Some("private".to_string()),
            content: snapshot("Unrelated", vec![managed_image(&upload)], Vec::new()),
        },
        None,
    )
    .await;
    assert!(
        unrelated.is_err(),
        "a copied asset ID grants no attachment authority"
    );

    let fork_draft = create_fork_draft(&pool, source.id, fork_owner.id, None)
        .await
        .unwrap();
    delete_user(&pool, source_owner.id).await.unwrap();
    let mut payload = fork_draft.decoded_snapshot().unwrap();
    payload.content.title = "Edited surviving fork".to_string();
    let fork_draft = update_recipe_draft(
        &pool,
        fork_draft.id,
        fork_owner.id,
        fork_draft.revision,
        &payload,
    )
    .await
    .unwrap();
    let fork = publish_recipe_draft(&pool, fork_draft.id, fork_owner.id, None)
        .await
        .unwrap();
    assert_eq!(fork.images[0].asset_id, Some(upload.id));

    let edit = create_edit_draft(&pool, fork.id, fork_owner.id)
        .await
        .unwrap();
    let mut edit_payload = edit.decoded_snapshot().unwrap();
    edit_payload.content.title = "Second surviving fork version".to_string();
    let edit = update_recipe_draft(&pool, edit.id, fork_owner.id, edit.revision, &edit_payload)
        .await
        .unwrap();
    let updated_fork = publish_recipe_draft(&pool, edit.id, fork_owner.id, None)
        .await
        .unwrap();
    assert_eq!(updated_fork.images[0].asset_id, Some(upload.id));
    let restored = restore_recipe_version(&pool, fork.id, fork_owner.id, 1, 2)
        .await
        .unwrap();
    assert_eq!(restored.images[0].asset_id, Some(upload.id));
    assert_eq!(
        sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT uploader_id FROM media_assets WHERE id = $1",
        )
        .bind(upload.id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        None
    );
    assert_eq!(
        claim_media_cleanup(&pool, upload.id).await.unwrap(),
        MediaDiscardDecision::Retained
    );
}

#[tokio::test]
async fn collection_and_avatar_references_share_the_asset_lifecycle() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let user = owner(&pool, "asset_other_holders").await;
    let cover = asset(user.id, MediaPurpose::CollectionCover, 41);
    let avatar = asset(user.id, MediaPurpose::Avatar, 42);
    register_media_asset(&pool, &cover).await.unwrap();
    register_media_asset(&pool, &avatar).await.unwrap();

    let collection = insert_collection(
        &pool,
        user.id,
        None,
        "Asset collection",
        None,
        crate::types::CollectionVisibility::Private,
        Some(&managed_image(&cover)),
    )
    .await
    .unwrap();
    replace_avatar_media_reference(&pool, user.id, avatar.id)
        .await
        .unwrap();

    assert_eq!(
        sqlx::query_scalar::<_, Uuid>(
            "SELECT asset_id FROM collection_media_refs WHERE collection_id = $1",
        )
        .bind(collection.id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        cover.id
    );
    assert_eq!(
        sqlx::query_scalar::<_, Uuid>("SELECT asset_id FROM avatar_media_refs WHERE user_id = $1",)
            .bind(user.id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        avatar.id
    );

    delete_user(&pool, user.id).await.unwrap();
    for asset_id in [cover.id, avatar.id] {
        assert_eq!(
            sqlx::query_scalar::<_, Option<Uuid>>(
                "SELECT uploader_id FROM media_assets WHERE id = $1",
            )
            .bind(asset_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
            None
        );
        assert!(matches!(
            claim_media_cleanup(&pool, asset_id).await.unwrap(),
            MediaDiscardDecision::Claimed(_)
        ));
    }
}

#[tokio::test]
async fn managed_read_surfaces_resolve_urls_from_the_asset_registry() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let user = owner(&pool, "asset_read_projection").await;
    let recipe_asset = asset(user.id, MediaPurpose::Recipe, 51);
    let nested_asset = asset(user.id, MediaPurpose::Recipe, 52);
    let cover_asset = asset(user.id, MediaPurpose::CollectionCover, 53);
    let avatar_asset = asset(user.id, MediaPurpose::Avatar, 54);
    for media_asset in [&recipe_asset, &nested_asset, &cover_asset, &avatar_asset] {
        register_media_asset(&pool, media_asset).await.unwrap();
    }

    let recipe = create_published_recipe(
        &pool,
        user.id,
        &diff::RecipeDraftPayload {
            initial_visibility: Some("private".to_string()),
            content: snapshot(
                "Registry-backed reads",
                vec![managed_image(&recipe_asset)],
                vec![RecipeStep {
                    text: "Cook".to_string(),
                    sub_steps: Vec::new(),
                    photos: vec![managed_image(&nested_asset)],
                }],
            ),
        },
        None,
    )
    .await
    .unwrap();
    let draft = create_edit_draft(&pool, recipe.id, user.id).await.unwrap();
    let collection = insert_collection(
        &pool,
        user.id,
        None,
        "Registry-backed collection",
        None,
        crate::types::CollectionVisibility::Private,
        Some(&managed_image(&cover_asset)),
    )
    .await
    .unwrap();
    add_recipe_to_collection(&pool, collection.id, user.id, recipe.id)
        .await
        .unwrap();
    replace_avatar_media_reference(&pool, user.id, avatar_asset.id)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE users SET avatar_url = 'https://stale.example/avatar.webp', \
             avatar_original_url = 'https://stale.example/avatar-original.webp' WHERE id = $1",
    )
    .bind(user.id)
    .execute(&pool)
    .await
    .unwrap();

    let stale_image = serde_json::json!([{
        "asset_id": recipe_asset.id,
        "url": "https://stale.example/recipe.webp",
        "thumb_url": "https://stale.example/recipe-thumb.webp",
        "edits": []
    }]);
    let stale_instructions = serde_json::json!([{
        "text": "Cook",
        "sub_steps": [],
        "photos": [{
            "asset_id": nested_asset.id,
            "url": "https://stale.example/nested.webp",
            "thumb_url": "https://stale.example/nested-thumb.webp",
            "edits": []
        }]
    }]);
    sqlx::query("UPDATE recipes SET images = $2, instructions = $3 WHERE id = $1")
        .bind(recipe.id)
        .bind(&stale_image)
        .bind(&stale_instructions)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE recipe_versions SET snapshot = jsonb_set(\
             jsonb_set(snapshot, '{images}', $2), '{instructions}', $3) \
         WHERE recipe_id = $1",
    )
    .bind(recipe.id)
    .bind(&stale_image)
    .bind(&stale_instructions)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE recipe_drafts SET snapshot = jsonb_set(\
             jsonb_set(snapshot, '{content,images}', $2), '{content,instructions}', $3) \
         WHERE id = $1",
    )
    .bind(draft.id)
    .bind(&stale_image)
    .bind(&stale_instructions)
    .execute(&pool)
    .await
    .unwrap();
    let stale_cover = serde_json::json!({
        "asset_id": cover_asset.id,
        "url": "https://stale.example/cover.webp",
        "thumb_url": "https://stale.example/cover-thumb.webp",
        "edits": []
    });
    sqlx::query("UPDATE collections SET image = $2 WHERE id = $1")
        .bind(collection.id)
        .bind(stale_cover)
        .execute(&pool)
        .await
        .unwrap();

    let visibility_response = update_recipe_visibility(&pool, recipe.id, user.id, "unlisted")
        .await
        .unwrap();
    assert_eq!(visibility_response.images[0].url, recipe_asset.full_url);
    assert_eq!(
        visibility_response.author_avatar_url.as_deref(),
        Some(avatar_asset.full_url.as_str())
    );
    let collection_visibility_response = update_collection_visibility(
        &pool,
        collection.id,
        user.id,
        crate::types::CollectionVisibility::Unlisted,
    )
    .await
    .unwrap();
    assert_eq!(
        collection_visibility_response.image.unwrap().url,
        cover_asset.full_url
    );
    let moved_collection = move_collection(&pool, collection.id, user.id, None)
        .await
        .unwrap();
    assert_eq!(moved_collection.image.unwrap().url, cover_asset.full_url);
    let profile_response = update_user_profile(&pool, user.id, "Updated Owner", None)
        .await
        .unwrap();
    assert_eq!(
        profile_response.avatar_url.as_deref(),
        Some(avatar_asset.full_url.as_str())
    );

    let loaded_recipe = get_recipe_by_id(&pool, recipe.id).await.unwrap().unwrap();
    assert_eq!(loaded_recipe.images[0].url, recipe_asset.full_url);
    assert_eq!(
        loaded_recipe.instructions[0].photos[0].thumb_url.as_deref(),
        Some(nested_asset.thumbnail_url.as_str())
    );
    assert_eq!(
        loaded_recipe.author_avatar_url.as_deref(),
        Some(avatar_asset.full_url.as_str())
    );

    let loaded_version = get_recipe_version(&pool, recipe.id, 1)
        .await
        .unwrap()
        .decoded_snapshot()
        .unwrap();
    assert_eq!(loaded_version.images[0].url, recipe_asset.full_url);
    assert_eq!(
        loaded_version.instructions[0].photos[0].url,
        nested_asset.full_url
    );
    let loaded_draft = get_recipe_draft(&pool, draft.id, user.id)
        .await
        .unwrap()
        .decoded_snapshot()
        .unwrap();
    assert_eq!(loaded_draft.content.images[0].url, recipe_asset.full_url);
    let history = get_recipe_version_summary_page(&pool, recipe.id, None, 20)
        .await
        .unwrap();
    assert_eq!(
        history.page.items[0].image_url.as_deref(),
        Some(recipe_asset.thumbnail_url.as_str())
    );
    let graph = get_initial_recipe_version_graph(&pool, recipe.id, Some(user.id))
        .await
        .unwrap();
    assert_eq!(
        graph.nodes[0].thumbnail_url(),
        Some(recipe_asset.thumbnail_url.as_str())
    );

    let loaded_collection = get_collection_by_id_and_owner(&pool, collection.id, user.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded_collection.image.unwrap().url, cover_asset.full_url);
    let collection_summaries = get_root_collections(&pool, user.id).await.unwrap();
    assert_eq!(
        collection_summaries[0].image.as_ref().unwrap().url,
        cover_asset.full_url
    );
    assert_eq!(
        collection_summaries[0].preview_images[0].url,
        recipe_asset.full_url
    );

    let loaded_user = get_user_by_id(&pool, user.id).await.unwrap().unwrap();
    assert_eq!(loaded_user.avatar_url, Some(avatar_asset.full_url.clone()));

    let recipe_search = search_recipes(
        &pool,
        user.id,
        &crate::types::RecipeSearchRequest {
            visibility: crate::types::RecipeSearchVisibility::MineOnly,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let searched_recipe = recipe_search
        .results
        .iter()
        .find(|result| result.recipe.id == recipe.id)
        .unwrap();
    assert_eq!(
        searched_recipe.recipe.image.as_ref().unwrap().url,
        recipe_asset.full_url
    );
    assert_eq!(
        searched_recipe.recipe.author_avatar_url.as_deref(),
        Some(avatar_asset.full_url.as_str())
    );
    let user_search = search_users(&pool, &crate::types::UserSearchRequest::default())
        .await
        .unwrap();
    let searched_user = user_search
        .results
        .iter()
        .find(|result| result.user.id == user.id)
        .unwrap();
    assert_eq!(
        searched_user.user.avatar_url.as_deref(),
        Some(avatar_asset.full_url.as_str())
    );
}

#[tokio::test]
async fn managed_recipe_reads_fail_closed_without_the_holder_reference() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let user = owner(&pool, "asset_missing").await;
    let recipe_asset = asset(user.id, MediaPurpose::Recipe, 61);
    register_media_asset(&pool, &recipe_asset).await.unwrap();
    let recipe = create_published_recipe(
        &pool,
        user.id,
        &diff::RecipeDraftPayload {
            initial_visibility: Some("private".to_string()),
            content: snapshot(
                "Missing reference",
                vec![managed_image(&recipe_asset)],
                Vec::new(),
            ),
        },
        None,
    )
    .await
    .unwrap();
    let draft = create_edit_draft(&pool, recipe.id, user.id).await.unwrap();
    let cover_asset = asset(user.id, MediaPurpose::CollectionCover, 62);
    let avatar_asset = asset(user.id, MediaPurpose::Avatar, 63);
    register_media_asset(&pool, &cover_asset).await.unwrap();
    register_media_asset(&pool, &avatar_asset).await.unwrap();
    let collection = insert_collection(
        &pool,
        user.id,
        None,
        "Missing reference collection",
        None,
        crate::types::CollectionVisibility::Private,
        Some(&managed_image(&cover_asset)),
    )
    .await
    .unwrap();
    replace_avatar_media_reference(&pool, user.id, avatar_asset.id)
        .await
        .unwrap();

    sqlx::query("DELETE FROM recipe_media_refs WHERE recipe_id = $1")
        .bind(recipe.id)
        .execute(&pool)
        .await
        .unwrap();

    assert!(matches!(
        get_recipe_by_id(&pool, recipe.id).await,
        Err(DbError::Query(sqlx::Error::Decode(_)))
    ));

    sqlx::query(
        "DELETE FROM recipe_version_media_refs WHERE recipe_version_id IN (\
             SELECT id FROM recipe_versions WHERE recipe_id = $1)",
    )
    .bind(recipe.id)
    .execute(&pool)
    .await
    .unwrap();
    let version = get_recipe_version(&pool, recipe.id, 1).await.unwrap();
    assert!(matches!(
        version.decoded_snapshot(),
        Err(DbError::InvalidRecipeSnapshot(_))
    ));

    sqlx::query("DELETE FROM recipe_draft_media_refs WHERE recipe_draft_id = $1")
        .bind(draft.id)
        .execute(&pool)
        .await
        .unwrap();
    let draft = get_recipe_draft(&pool, draft.id, user.id).await.unwrap();
    assert!(matches!(
        draft.decoded_snapshot(),
        Err(DbError::InvalidRecipeSnapshot(_))
    ));

    sqlx::query("DELETE FROM collection_media_refs WHERE collection_id = $1")
        .bind(collection.id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        get_collection_by_id_and_owner(&pool, collection.id, user.id).await,
        Err(DbError::Query(sqlx::Error::Decode(_)))
    ));

    sqlx::query("DELETE FROM avatar_media_refs WHERE user_id = $1")
        .bind(user.id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        get_user_by_id(&pool, user.id)
            .await
            .unwrap()
            .unwrap()
            .avatar_url,
        None
    );
}

#[tokio::test]
async fn external_avatar_urls_remain_readable_without_a_managed_reference() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let user = owner(&pool, "external_avatar").await;
    let external_url = "https://identity.example.test/avatar.png".to_string();
    update_avatar_url(&pool, user.id, Some(external_url.clone()))
        .await
        .unwrap();

    assert_eq!(
        get_user_by_id(&pool, user.id)
            .await
            .unwrap()
            .unwrap()
            .avatar_url,
        Some(external_url.clone())
    );
    let search = search_users(&pool, &crate::types::UserSearchRequest::default())
        .await
        .unwrap();
    assert_eq!(
        search
            .results
            .into_iter()
            .find(|result| result.user.id == user.id)
            .unwrap()
            .user
            .avatar_url,
        Some(external_url)
    );
}
