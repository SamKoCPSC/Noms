use super::*;
use crate::{db, types::CollectionVisibility};
use serde_json::{json, Value};
use uuid::Uuid;

async fn user(pool: &sqlx::PgPool, label: &str) -> db::User {
    let unique = crate::test_utils::uid();
    db::insert_user(
        pool,
        &format!("{label}_{unique}"),
        label,
        &format!("{label}_{unique}@example.test"),
        None,
    )
    .await
    .unwrap()
}

async fn tree(pool: &sqlx::PgPool) -> (db::User, Uuid, Vec<Uuid>) {
    let owner = user(pool, "tree").await;
    let other = user(pool, "shared").await;
    let root = db::insert_collection(
        pool,
        owner.id,
        None,
        "Family recipes",
        Some("An ordered collection"),
        CollectionVisibility::Public,
        None,
    )
    .await
    .unwrap();
    let child = db::insert_collection(
        pool,
        owner.id,
        Some(root.id),
        "Weeknights",
        None,
        CollectionVisibility::Unlisted,
        None,
    )
    .await
    .unwrap();
    let empty = db::insert_collection(
        pool,
        owner.id,
        Some(root.id),
        "Empty album",
        Some("Keep this empty collection"),
        CollectionVisibility::Private,
        None,
    )
    .await
    .unwrap();
    let grandchild = db::insert_collection(
        pool,
        owner.id,
        Some(child.id),
        "Soups",
        None,
        CollectionVisibility::Private,
        None,
    )
    .await
    .unwrap();
    let mut content = crate::test_utils::test_recipe_snapshot("Bean soup & bread 🥣");
    content.commentary = Some("A note\nwith two lines".into());
    content.prep_time_minutes = Some(15);
    content.servings = Some(4);
    content.tags = vec!["Weeknight".into(), "Vegetarian".into()];
    content.instructions[0]
        .sub_steps
        .push(crate::types::RecipeStep {
            text: "Stir gently".into(),
            sub_steps: vec![],
            photos: vec![],
        });
    let first = db::create_published_recipe(
        pool,
        owner.id,
        &db::diff::RecipeDraftPayload {
            content,
            initial_visibility: Some("private".into()),
        },
        None,
    )
    .await
    .unwrap();
    let second =
        crate::test_utils::create_test_published_recipe(pool, other.id, "Shared bread").await;
    db::update_recipe_visibility(pool, second.id, other.id, "public")
        .await
        .unwrap();
    for (collection, recipe) in [
        (root.id, second.id),
        (root.id, first.id),
        (child.id, first.id),
        (child.id, second.id),
        (grandchild.id, first.id),
    ] {
        db::add_recipe_to_collection(pool, collection, owner.id, recipe)
            .await
            .unwrap();
    }
    assert!(db::get_child_collections(pool, root.id, owner.id)
        .await
        .unwrap()
        .iter()
        .any(|c| c.id == empty.id));
    (owner, root.id, vec![first.id, second.id])
}

fn normalized(json: &str) -> Value {
    let parsed = crate::collection_json::parse(json).unwrap();
    let collections = &parsed.snapshot.collections;
    let recipes = &parsed.snapshot.recipes;
    let nodes = collections.iter().map(|node| json!({
        "name":node.name, "description":node.description,
        "parent":node.parent_collection.as_ref().map(|p| collections.iter().position(|c| &c.portable_id == p).unwrap()),
        "recipes":node.recipes.iter().map(|id| recipes.iter().position(|r| &r.portable_id == id).unwrap()).collect::<Vec<_>>(),
        "image":node.image,
    })).collect::<Vec<_>>();
    json!({"collections":nodes, "recipes":recipes.iter().map(|r| &r.snapshot).collect::<Vec<_>>()})
}

async fn counts(pool: &sqlx::PgPool, owner: Uuid) -> (i64, i64, i64, i64, i64) {
    sqlx::query_as("SELECT (SELECT COUNT(*) FROM collections WHERE owner_id=$1), (SELECT COUNT(*) FROM recipes WHERE user_id=$1), (SELECT COUNT(*) FROM recipe_versions v JOIN recipes r ON r.id=v.recipe_id WHERE r.user_id=$1), (SELECT COUNT(*) FROM recipe_drafts WHERE user_id=$1), (SELECT COUNT(*) FROM collection_recipes cr JOIN collections c ON c.id=cr.collection_id WHERE c.owner_id=$1)")
        .bind(owner).fetch_one(pool).await.unwrap()
}

#[tokio::test]
async fn collection_json_roundtrip_preserves_private_copies_order_shared_memberships_and_saved_content(
) {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let (owner, root, original_ids) = tree(&pool).await;
    let draft = crate::test_utils::create_test_edit_draft(&pool, owner.id, original_ids[0]).await;
    let mut payload = draft.decoded_snapshot().unwrap();
    payload.content.title = "Unpublished secret".into();
    db::update_recipe_draft(&pool, draft.id, owner.id, draft.revision, &payload)
        .await
        .unwrap();
    let original = db::export_collection_json(&pool, owner.id, root)
        .await
        .unwrap();
    assert!(!original.contains("Unpublished secret"));
    let parsed = crate::collection_json::parse(&original).unwrap();
    assert_eq!(
        (
            parsed.snapshot.collections.len(),
            parsed.snapshot.recipes.len()
        ),
        (4, 2)
    );
    let importer = user(&pool, "import").await;
    let storage = crate::storage::StorageClient::for_test();
    let imported = import_for_user(&pool, &storage, importer.id, &original, &[])
        .await
        .unwrap();
    assert_eq!(imported.owner_id, importer.id);
    assert!(imported.image_reviews.is_empty());
    assert_eq!(counts(&pool, importer.id).await, (4, 2, 2, 0, 5));
    assert_eq!(
        normalized(&original),
        normalized(
            &db::export_collection_json(&pool, importer.id, imported.collection_id)
                .await
                .unwrap()
        )
    );
    let visibility: Vec<String> = sqlx::query_scalar("SELECT visibility FROM collections WHERE owner_id=$1 UNION ALL SELECT visibility FROM recipes WHERE user_id=$1")
        .bind(importer.id).fetch_all(&pool).await.unwrap();
    assert_eq!(visibility, vec!["private"; 6]);
    let new_ids: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM recipes WHERE user_id=$1")
        .bind(importer.id)
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(new_ids.iter().all(|id| !original_ids.contains(id)));
    let again = import_for_user(&pool, &storage, importer.id, &original, &[])
        .await
        .unwrap();
    assert_ne!(again.collection_id, imported.collection_id);
    assert!(again.warnings.iter().any(|w| w.contains("imported")));
    assert_eq!(counts(&pool, importer.id).await, (8, 4, 4, 0, 10));
}

#[tokio::test]
async fn collection_json_export_checks_owner_and_fails_on_revoked_recipe_access() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let (owner, root, ids) = tree(&pool).await;
    let other = user(&pool, "stranger").await;
    assert!(db::export_collection_json(&pool, other.id, root)
        .await
        .is_err());
    assert!(db::export_collection_json(&pool, owner.id, Uuid::new_v4())
        .await
        .is_err());
    sqlx::query("UPDATE recipes SET visibility='private' WHERE id=$1")
        .bind(ids[1])
        .execute(&pool)
        .await
        .unwrap();
    let error = db::export_collection_json(&pool, owner.id, root)
        .await
        .unwrap_err();
    assert!(error.contains("no longer accessible"));
    assert!(!error.contains("Shared bread"));
}

#[tokio::test]
async fn collection_json_import_validation_and_database_failures_leave_no_partial_writes() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let (owner, root, _) = tree(&pool).await;
    let original = db::export_collection_json(&pool, owner.id, root)
        .await
        .unwrap();
    let importer = user(&pool, "rollback").await;
    let storage = crate::storage::StorageClient::for_test();
    let before = counts(&pool, importer.id).await;
    let mut damaged: Value = serde_json::from_str(&original).unwrap();
    damaged["snapshot"]["collections"][0]["name"] = json!("Tampered");
    assert!(
        import_for_user(&pool, &storage, importer.id, &damaged.to_string(), &[])
            .await
            .is_err()
    );
    assert_eq!(counts(&pool, importer.id).await, before);
    sqlx::query("CREATE FUNCTION reject_portability_child() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.name='Weeknights' THEN RAISE EXCEPTION 'Synthetic late collection insert failure'; END IF; RETURN NEW; END $$")
        .execute(&pool).await.unwrap();
    sqlx::query("CREATE TRIGGER reject_portability_child BEFORE INSERT ON collections FOR EACH ROW EXECUTE FUNCTION reject_portability_child()")
        .execute(&pool).await.unwrap();
    let error = import_for_user(&pool, &storage, importer.id, &original, &[])
        .await
        .unwrap_err();
    assert!(!error.to_string().contains("Synthetic"));
    assert_eq!(counts(&pool, importer.id).await, before);
}

#[tokio::test]
async fn collection_json_import_reuses_authorized_recipe_images_and_keeps_missing_images_reviewable(
) {
    use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let mock = MockServer::start().await;
    Mock::given(method("HEAD"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-length", "24"))
        .mount(&mock)
        .await;
    let storage = crate::storage::StorageClient::for_test_with_endpoint_and_public_url(
        &mock.uri(),
        "https://saved.example/media",
    );
    let (recipe, owner) = super::super::recipe_archive::tests::source(&pool, &storage).await;
    let root = db::insert_collection(
        &pool,
        owner,
        None,
        "Images",
        None,
        CollectionVisibility::Private,
        None,
    )
    .await
    .unwrap();
    db::add_recipe_to_collection(&pool, root.id, owner, recipe.id)
        .await
        .unwrap();
    let original = db::export_collection_json(&pool, owner, root.id)
        .await
        .unwrap();
    let available = vec!["https://external.example/photo.webp".into()];
    let reused = import_for_user(&pool, &storage, owner, &original, &available)
        .await
        .unwrap();
    assert!(reused.image_reviews.is_empty());
    assert_eq!(normalized(&original), {
        let mut value = normalized(
            &db::export_collection_json(&pool, owner, reused.collection_id)
                .await
                .unwrap(),
        );
        value["collections"][0]["name"] = json!("Images");
        value
    });
    mock.reset().await;
    Mock::given(method("HEAD"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock)
        .await;
    let missing = import_for_user(&pool, &storage, owner, &original, &[])
        .await
        .unwrap();
    assert_eq!(missing.image_reviews.len(), 1);
    assert_eq!(missing.image_reviews[0].image_count, 4);
    let draft = db::get_recipe_draft(&pool, missing.image_reviews[0].draft_id, owner)
        .await
        .unwrap();
    assert!(draft.recipe_id.is_some());
    assert_eq!(db::json_import_review(&draft).unwrap().images.len(), 4);
    let saved = db::get_recipe_by_id_and_owner(&pool, missing.image_reviews[0].recipe_id, owner)
        .await
        .unwrap();
    assert!(saved.images.is_empty());
    assert_eq!(saved.title, recipe.title);
    // A forged source identity does not authorize somebody else's private assets.
    let other = user(&pool, "imagecopy").await;
    mock.reset().await;
    let denied = import_for_user(&pool, &storage, other.id, &original, &[])
        .await
        .unwrap();
    assert_eq!(denied.image_reviews[0].image_count, 4);
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn collection_json_cover_reuse_is_uploader_only_and_follows_storage_availability() {
    use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let owner = user(&pool, "cover").await;
    let mock = MockServer::start().await;
    Mock::given(method("HEAD"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-length", "24"))
        .mount(&mock)
        .await;
    let storage = crate::storage::StorageClient::for_test_with_endpoint_and_public_url(
        &mock.uri(),
        "https://saved.example/media",
    );
    let id = Uuid::new_v4();
    let full_key = format!("collections/{}/{id}_full.webp", owner.id);
    let thumbnail_key = format!("collections/{}/{id}_thumb.webp", owner.id);
    let image = crate::types::RecipeImage::managed(
        id,
        storage.object_url(&full_key),
        Some(storage.object_url(&thumbnail_key)),
        crate::types::RecipeImageEdit::default_rect_16x9(),
    );
    db::register_media_asset(
        &pool,
        &db::NewMediaAsset {
            id,
            uploader_id: owner.id,
            purpose: db::MediaPurpose::CollectionCover,
            full_url: image.url.clone(),
            thumbnail_url: image.thumb_url.clone().unwrap(),
            original_url: None,
            full_key,
            thumbnail_key,
            original_key: None,
            capability_digest: Some(vec![1; 32]),
            expires_at: Some(chrono::Utc::now() + chrono::Duration::minutes(15)),
        },
    )
    .await
    .unwrap();
    let root = db::insert_collection(
        &pool,
        owner.id,
        None,
        "Cover",
        None,
        CollectionVisibility::Private,
        Some(&image),
    )
    .await
    .unwrap();
    let original = db::export_collection_json(&pool, owner.id, root.id)
        .await
        .unwrap();
    let reused = import_for_user(&pool, &storage, owner.id, &original, &[])
        .await
        .unwrap();
    let copy = db::get_collection_by_id_and_owner(&pool, reused.collection_id, owner.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(copy.image, Some(image));
    let other = user(&pool, "covercopy").await;
    mock.reset().await;
    let skipped = import_for_user(&pool, &storage, other.id, &original, &[])
        .await
        .unwrap();
    assert_eq!(skipped.warnings.len(), 1);
    assert!(skipped.image_reviews.is_empty());
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn collection_json_capacity_checks_do_not_block_text_imports_or_lose_image_review() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let (owner, root, _) = tree(&pool).await;
    let json = db::export_collection_json(&pool, owner.id, root)
        .await
        .unwrap();
    let importer = user(&pool, "capacity").await;
    let payload = serde_json::to_value(db::diff::RecipeDraftPayload {
        content: crate::test_utils::test_recipe_snapshot("Existing draft"),
        initial_visibility: Some("private".into()),
    })
    .unwrap();
    sqlx::query(
        "INSERT INTO recipe_drafts (user_id, snapshot) SELECT $1,$2 FROM generate_series(1,$3)",
    )
    .bind(importer.id)
    .bind(payload)
    .bind(crate::security::MAX_ACTIVE_DRAFTS_PER_USER as i32)
    .execute(&pool)
    .await
    .unwrap();
    let storage = crate::storage::StorageClient::for_test();
    assert!(import_for_user(&pool, &storage, importer.id, &json, &[])
        .await
        .is_ok());
    let before = counts(&pool, importer.id).await;
    let (recipe, image_owner) = super::super::recipe_archive::tests::source(&pool, &storage).await;
    let image_root = db::insert_collection(
        &pool,
        image_owner,
        None,
        "Needs review",
        None,
        CollectionVisibility::Private,
        None,
    )
    .await
    .unwrap();
    db::add_recipe_to_collection(&pool, image_root.id, image_owner, recipe.id)
        .await
        .unwrap();
    let image_json = db::export_collection_json(&pool, image_owner, image_root.id)
        .await
        .unwrap();
    assert!(
        import_for_user(&pool, &storage, importer.id, &image_json, &[])
            .await
            .is_err()
    );
    assert_eq!(counts(&pool, importer.id).await, before);
    let full = user(&pool, "full").await;
    sqlx::query("INSERT INTO collections (owner_id,name) SELECT $1,'Existing '||series FROM generate_series(1,$2) series")
        .bind(full.id).bind(crate::security::MAX_COLLECTION_CHILDREN as i32).execute(&pool).await.unwrap();
    let before = counts(&pool, full.id).await;
    assert!(import_for_user(&pool, &storage, full.id, &json, &[])
        .await
        .is_err());
    assert_eq!(counts(&pool, full.id).await, before);
}

#[tokio::test]
#[ignore = "Manual browser fixture; requires NOMS_COLLECTION_BROWSER_FIXTURE_DIR"]
async fn collection_json_browser_fixture() {
    let dir = std::path::PathBuf::from(
        std::env::var("NOMS_COLLECTION_BROWSER_FIXTURE_DIR").expect("fixture directory"),
    );
    let (database, pool) = crate::test_utils::setup_test_db().await;
    let (owner, root, _) = tree(&pool).await;
    let importer = user(&pool, "browser").await;
    crate::auth::session::set_test_secret("synthetic-collection-browser-session-only-00000000");
    let owner_token = crate::auth::session::create_session(&pool, owner.id)
        .await
        .unwrap();
    let importer_token = crate::auth::session::create_session(&pool, importer.id)
        .await
        .unwrap();
    let file = db::export_collection_json(&pool, owner.id, root)
        .await
        .unwrap();
    let storage = crate::storage::StorageClient::for_test_with_endpoint_and_public_url(
        "http://127.0.0.1:39101",
        "http://127.0.0.1:39101/noms-media",
    );
    let (recipe, image_owner) = super::super::recipe_archive::tests::source(&pool, &storage).await;
    let image_root = db::insert_collection(
        &pool,
        image_owner,
        None,
        "Recipes with image checks",
        None,
        CollectionVisibility::Private,
        None,
    )
    .await
    .unwrap();
    db::add_recipe_to_collection(&pool, image_root.id, image_owner, recipe.id)
        .await
        .unwrap();
    let image_file = db::export_collection_json(&pool, image_owner, image_root.id)
        .await
        .unwrap();
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("collection.json"), file).unwrap();
    std::fs::write(dir.join("images.json"), image_file).unwrap();
    let path = dir.join("fixture.json");
    std::fs::write(&path, serde_json::to_vec(&json!({"database_url": database.connection_uri(), "collection_id": root,
        "owner_id": owner.id, "importer_id": importer.id, "owner_token": owner_token, "importer_token": importer_token})).unwrap()).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    for _ in 0..1800 {
        if dir.join("stop").exists() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
