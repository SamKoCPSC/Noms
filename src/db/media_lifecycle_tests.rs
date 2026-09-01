use super::*;
use chrono::{Duration, Utc};
use std::time::Duration as StdDuration;

async fn owner(pool: &PgPool, prefix: &str) -> User {
    let suffix = crate::test_utils::uid();
    insert_user(
        pool,
        &format!("{prefix}_{suffix}"),
        "Media Owner",
        &format!("{prefix}_{suffix}@example.test"),
        None,
    )
    .await
    .unwrap()
}

fn family(owner_id: Uuid, purpose: MediaPurpose, marker: u8) -> NewMediaUploadFamily {
    let object_id = Uuid::new_v4();
    let namespace = match purpose {
        MediaPurpose::Recipe => "recipes",
        MediaPurpose::CollectionCover => "collections",
    };
    let full_key = format!("{namespace}/{owner_id}/{object_id}_full.webp");
    let thumbnail_key = format!("{namespace}/{owner_id}/{object_id}_thumb.webp");
    NewMediaUploadFamily {
        owner_id,
        purpose,
        full_url: format!("https://media.example.test/{full_key}"),
        thumbnail_url: format!("https://media.example.test/{thumbnail_key}"),
        full_key,
        thumbnail_key,
        capability_digest: vec![marker; 32],
        expires_at: Utc::now() + Duration::minutes(10),
    }
}

#[tokio::test]
async fn account_deletion_tombstones_tracked_and_avatar_media_before_cascades() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let user = owner(&pool, "account_media").await;
    let recipe = family(user.id, MediaPurpose::Recipe, 31);
    let collection = family(user.id, MediaPurpose::CollectionCover, 32);
    register_media_upload_family(&pool, &recipe).await.unwrap();
    register_media_upload_family(&pool, &collection)
        .await
        .unwrap();

    let avatar_object = Uuid::new_v4();
    let avatar_url = format!(
        "https://media.example.test/avatars/{}/{avatar_object}.webp",
        user.id
    );
    let avatar_original_url = format!(
        "https://media.example.test/avatars/{}/{avatar_object}-original",
        user.id
    );
    let avatar_full_key = format!("avatars/{}/{avatar_object}.webp", user.id);
    let avatar_thumbnail_key = format!("avatars/{}/{avatar_object}-thumb.webp", user.id);
    let avatar_original_key = format!("avatars/{}/{avatar_object}-original", user.id);
    sqlx::query("UPDATE users SET avatar_url = $2, avatar_original_url = $3 WHERE id = $1")
        .bind(user.id)
        .bind(&avatar_url)
        .bind(&avatar_original_url)
        .execute(&pool)
        .await
        .unwrap();

    let storage = crate::storage::StorageClient::for_test_with_endpoint_and_public_url(
        "http://127.0.0.1:9",
        "https://media.example.test",
    );
    delete_user_with_media_cleanup(&pool, user.id, &storage)
        .await
        .unwrap();

    let rows = sqlx::query_as::<
        _,
        (
            AccountMediaPurpose,
            Uuid,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            String,
        ),
    >(
        "SELECT purpose, former_owner_id, full_url, thumbnail_url, original_url, \
                full_key, thumbnail_key, original_key, lifecycle \
         FROM account_media_cleanup_tombstones WHERE former_owner_id = $1 ORDER BY purpose",
    )
    .bind(user.id)
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(rows.len(), 3);
    assert!(rows
        .iter()
        .all(|row| row.1 == user.id && row.8 == "pending"));
    assert!(rows.iter().any(|row| {
        row.0 == AccountMediaPurpose::Recipe
            && row.2.as_deref() == Some(recipe.full_url.as_str())
            && row.5.as_deref() == Some(recipe.full_key.as_str())
            && row.6.as_deref() == Some(recipe.thumbnail_key.as_str())
    }));
    assert!(rows.iter().any(|row| {
        row.0 == AccountMediaPurpose::CollectionCover
            && row.2.as_deref() == Some(collection.full_url.as_str())
            && row.5.as_deref() == Some(collection.full_key.as_str())
            && row.6.as_deref() == Some(collection.thumbnail_key.as_str())
    }));
    assert!(rows.iter().any(|row| {
        row.0 == AccountMediaPurpose::Avatar
            && row.2.as_deref() == Some(avatar_url.as_str())
            && row.4.as_deref() == Some(avatar_original_url.as_str())
            && row.5.as_deref() == Some(avatar_full_key.as_str())
            && row.6.as_deref() == Some(avatar_thumbnail_key.as_str())
            && row.7.as_deref() == Some(avatar_original_key.as_str())
    }));

    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM media_upload_families WHERE owner_id = $1",
        )
        .bind(user.id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0,
        "the user-owned source rows may cascade only after cleanup authority is copied"
    );
}

#[tokio::test]
async fn account_deletion_never_stages_foreign_avatar_keys() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let user = owner(&pool, "account_avatar_owner").await;
    let victim_id = Uuid::new_v4();
    let object_id = Uuid::new_v4();
    let storage = crate::storage::StorageClient::for_test_with_endpoint_and_public_url(
        "http://127.0.0.1:9",
        "https://media.example.test",
    );
    let victim_full = storage.object_url(&format!("avatars/{victim_id}/{object_id}.webp"));
    let victim_original = storage.object_url(&format!("avatars/{victim_id}/{object_id}-original"));
    sqlx::query("UPDATE users SET avatar_url = $2, avatar_original_url = $3 WHERE id = $1")
        .bind(user.id)
        .bind(victim_full)
        .bind(victim_original)
        .execute(&pool)
        .await
        .unwrap();

    delete_user_with_media_cleanup(&pool, user.id, &storage)
        .await
        .unwrap();

    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM account_media_cleanup_tombstones WHERE former_owner_id = $1",
        )
        .bind(user.id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0,
        "foreign URLs must never become durable deletion authority"
    );
}

#[tokio::test]
async fn account_media_cleanup_claim_is_leased_and_failure_is_immediately_retryable() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let user = owner(&pool, "account_claim").await;
    let upload = family(user.id, MediaPurpose::Recipe, 34);
    register_media_upload_family(&pool, &upload).await.unwrap();
    delete_user(&pool, user.id).await.unwrap();
    let tombstone_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM account_media_cleanup_tombstones WHERE former_owner_id = $1",
    )
    .bind(user.id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(matches!(
        claim_account_media_cleanup(&pool, tombstone_id)
            .await
            .unwrap(),
        AccountMediaCleanupDecision::Claimed(_)
    ));
    assert_eq!(
        claim_account_media_cleanup(&pool, tombstone_id)
            .await
            .unwrap(),
        AccountMediaCleanupDecision::Busy,
        "a fresh lease must prevent concurrent deletion"
    );

    finish_account_media_cleanup(&pool, tombstone_id, Some("storage unavailable"))
        .await
        .unwrap();
    assert!(matches!(
        claim_account_media_cleanup(&pool, tombstone_id)
            .await
            .unwrap(),
        AccountMediaCleanupDecision::Claimed(_)
    ));
}

#[tokio::test]
async fn discard_capability_is_owner_bound_one_use_and_deletes_only_its_exact_pair() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let first = owner(&pool, "media_cap_first").await;
    let second = owner(&pool, "media_cap_second").await;
    let upload = family(first.id, MediaPurpose::Recipe, 7);
    register_media_upload_family(&pool, &upload).await.unwrap();

    assert_eq!(
        claim_media_discard(&pool, second.id, &upload.capability_digest)
            .await
            .unwrap(),
        MediaDiscardDecision::Denied
    );

    let foreign_reference = RecipeImage::new(
        upload.full_url.clone(),
        Some(upload.thumbnail_url.clone()),
        crate::types::RecipeImageEdit::default_rect_16x9(),
    );
    insert_recipe(
        &pool,
        second.id,
        "Foreign reference",
        None,
        None,
        None,
        None,
        None,
        &[],
        &[],
        &[],
        std::slice::from_ref(&foreign_reference),
        "private",
    )
    .await
    .unwrap();
    insert_collection(
        &pool,
        first.id,
        None,
        "Wrong kind",
        None,
        crate::types::CollectionVisibility::Private,
        Some(&foreign_reference),
    )
    .await
    .unwrap();

    let claim = match claim_media_discard(&pool, first.id, &upload.capability_digest)
        .await
        .unwrap()
    {
        MediaDiscardDecision::Claimed(claim) => claim,
        decision => panic!("unreferenced upload should be claimable, got {decision:?}"),
    };
    assert_eq!(claim.full_key, upload.full_key);
    assert_eq!(claim.thumbnail_key, upload.thumbnail_key);
    assert_eq!(claim.purpose, MediaPurpose::Recipe);

    assert_eq!(
        claim_media_discard(&pool, first.id, &upload.capability_digest)
            .await
            .unwrap(),
        MediaDiscardDecision::Denied,
        "a capability must authorize only one discard attempt"
    );
}

#[tokio::test]
async fn retain_wins_a_concurrent_discard_and_makes_the_capability_non_destructive() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let user = owner(&pool, "media_race").await;
    let upload = family(user.id, MediaPurpose::Recipe, 11);
    register_media_upload_family(&pool, &upload).await.unwrap();

    let mut save = pool.begin().await.unwrap();
    retain_media_urls(
        &mut save,
        user.id,
        MediaPurpose::Recipe,
        &[upload.full_url.clone(), upload.thumbnail_url.clone()],
    )
    .await
    .unwrap();

    let discard_pool = pool.clone();
    let digest = upload.capability_digest.clone();
    let discard =
        tokio::spawn(async move { claim_media_discard(&discard_pool, user.id, &digest).await });
    tokio::time::sleep(StdDuration::from_millis(50)).await;
    assert!(
        !discard.is_finished(),
        "discard must wait for the save lock"
    );

    save.commit().await.unwrap();
    assert_eq!(
        discard.await.unwrap().unwrap(),
        MediaDiscardDecision::Retained,
        "a retained capability must never authorize storage deletion"
    );
}

#[tokio::test]
async fn inherited_fork_reference_keeps_source_media_after_source_deletion() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let source_owner = owner(&pool, "media_source").await;
    let fork_owner = owner(&pool, "media_fork").await;
    let upload = family(source_owner.id, MediaPurpose::Recipe, 19);
    register_media_upload_family(&pool, &upload).await.unwrap();

    let image = RecipeImage::new(
        upload.full_url.clone(),
        Some(upload.thumbnail_url.clone()),
        crate::types::RecipeImageEdit::default_rect_16x9(),
    );
    let source = create_published_recipe(
        &pool,
        source_owner.id,
        &crate::db::diff::RecipeDraftPayload {
            initial_visibility: Some("public".to_string()),
            content: crate::db::diff::RecipeSnapshot {
                title: "Source".to_string(),
                description: Some("description".to_string()),
                commentary: None,
                prep_time_minutes: Some(5),
                cook_time_minutes: Some(10),
                servings: Some(2),
                ingredients: vec![crate::types::RecipeIngredient {
                    amount: "1".to_string(),
                    unit: "cup".to_string(),
                    name: "beans".to_string(),
                }],
                instructions: vec![crate::types::RecipeStep {
                    text: "Cook".to_string(),
                    sub_steps: vec![],
                    photos: vec![],
                }],
                equipment: vec![crate::types::RecipeEquipment {
                    name: "Pot".to_string(),
                }],
                images: vec![image],
                tags: vec!["test".to_string()],
            },
        },
        None,
    )
    .await
    .unwrap();

    let fork_draft = create_fork_draft(&pool, source.id, fork_owner.id, None)
        .await
        .unwrap();
    assert!(fork_draft
        .decoded_snapshot()
        .unwrap()
        .content
        .images
        .iter()
        .any(|candidate| candidate.url == upload.full_url));
    delete_recipe(&pool, source.id, source_owner.id)
        .await
        .unwrap();

    assert_eq!(
        claim_retained_media_cleanup(&pool, source_owner.id, &upload.full_url)
            .await
            .unwrap(),
        MediaDiscardDecision::Retained,
        "a fork draft is an authorized live reference even after its source is gone"
    );
}

#[tokio::test]
async fn account_cleanup_retains_media_referenced_by_a_surviving_fork() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let source_owner = owner(&pool, "account_source").await;
    let fork_owner = owner(&pool, "account_fork").await;
    let upload = family(source_owner.id, MediaPurpose::Recipe, 33);
    register_media_upload_family(&pool, &upload).await.unwrap();

    let image = RecipeImage::new(
        upload.full_url.clone(),
        Some(upload.thumbnail_url.clone()),
        crate::types::RecipeImageEdit::default_rect_16x9(),
    );
    let source = create_published_recipe(
        &pool,
        source_owner.id,
        &crate::db::diff::RecipeDraftPayload {
            initial_visibility: Some("public".to_string()),
            content: crate::db::diff::RecipeSnapshot {
                title: "Account deletion source".to_string(),
                description: Some("description".to_string()),
                commentary: None,
                prep_time_minutes: Some(5),
                cook_time_minutes: Some(10),
                servings: Some(2),
                ingredients: vec![crate::types::RecipeIngredient {
                    amount: "1".to_string(),
                    unit: "cup".to_string(),
                    name: "beans".to_string(),
                }],
                instructions: vec![crate::types::RecipeStep {
                    text: "Cook".to_string(),
                    sub_steps: vec![],
                    photos: vec![],
                }],
                equipment: vec![crate::types::RecipeEquipment {
                    name: "Pot".to_string(),
                }],
                images: vec![image],
                tags: vec![],
            },
        },
        None,
    )
    .await
    .unwrap();
    create_fork_draft(&pool, source.id, fork_owner.id, None)
        .await
        .unwrap();

    delete_user(&pool, source_owner.id).await.unwrap();
    let tombstone_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM account_media_cleanup_tombstones \
         WHERE former_owner_id = $1 AND full_url = $2",
    )
    .bind(source_owner.id)
    .bind(&upload.full_url)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        claim_account_media_cleanup(&pool, tombstone_id)
            .await
            .unwrap(),
        AccountMediaCleanupDecision::Retained,
        "a surviving fork must keep inherited source media after account deletion"
    );
}

#[test]
fn client_discard_api_is_capability_only() {
    let upload_api = include_str!("../api/upload.rs");
    assert!(!upload_api.contains("pub async fn delete_image(full_url"));
    assert!(!upload_api.contains("pub async fn delete_unsaved_collection_cover(full_url"));
    assert!(upload_api.contains("pub async fn discard_upload(discard_capability"));
}
