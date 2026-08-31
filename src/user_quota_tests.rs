use crate::security::{
    MAX_ACTIVE_DRAFTS_PER_USER, MAX_COLLECTIONS_PER_USER, MAX_COLLECTION_CHILDREN,
    MAX_COLLECTION_DEPTH, MAX_COLLECTION_MEMBERSHIPS, MAX_PUBLIC_TAGS, MAX_VERSIONS_PER_RECIPE,
    MAX_VERSIONS_PER_USER,
};
use crate::types::{CollectionVisibility, RecipeIngredient, RecipeStep};

fn draft_payload(title: &str) -> crate::db::diff::RecipeDraftPayload {
    crate::db::diff::RecipeDraftPayload {
        content: crate::db::diff::RecipeSnapshot {
            title: title.to_string(),
            description: None,
            commentary: None,
            prep_time_minutes: None,
            cook_time_minutes: None,
            servings: None,
            ingredients: vec![RecipeIngredient {
                amount: "1".to_string(),
                unit: "cup".to_string(),
                name: "stock".to_string(),
            }],
            instructions: vec![RecipeStep {
                text: "Simmer".to_string(),
                sub_steps: Vec::new(),
                photos: Vec::new(),
            }],
            equipment: Vec::new(),
            images: Vec::new(),
            tags: Vec::new(),
        },
        initial_visibility: Some("private".to_string()),
    }
}

#[tokio::test]
async fn collection_total_quota_fails_without_partial_write_and_is_user_scoped() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let saturated_user = crate::test_utils::insert_test_user(&pool).await;
    let other_user = crate::test_utils::insert_test_user(&pool).await;

    sqlx::query(
        "INSERT INTO collections (owner_id, name, sort_order) \
         SELECT $1, 'quota-' || value::text, value \
         FROM generate_series(1, $2) AS value",
    )
    .bind(saturated_user.id)
    .bind(i32::try_from(MAX_COLLECTIONS_PER_USER).unwrap())
    .execute(&pool)
    .await
    .unwrap();

    assert!(crate::db::insert_collection(
        &pool,
        saturated_user.id,
        None,
        "one too many",
        None,
        CollectionVisibility::Private,
        None,
    )
    .await
    .is_err());
    let saturated_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM collections WHERE owner_id = $1")
            .bind(saturated_user.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        saturated_count,
        i64::try_from(MAX_COLLECTIONS_PER_USER).unwrap()
    );

    assert!(crate::db::insert_collection(
        &pool,
        other_user.id,
        None,
        "ordinary collection",
        None,
        CollectionVisibility::Private,
        None,
    )
    .await
    .is_ok());
}

#[tokio::test]
async fn collection_depth_and_direct_child_quotas_are_transactional() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let depth_owner = crate::test_utils::insert_test_user(&pool).await;
    let mut parent_id = None;
    for depth in 1..=MAX_COLLECTION_DEPTH {
        let collection = crate::db::insert_collection(
            &pool,
            depth_owner.id,
            parent_id,
            &format!("depth-{depth}"),
            None,
            CollectionVisibility::Private,
            None,
        )
        .await
        .unwrap();
        parent_id = Some(collection.id);
    }
    assert!(crate::db::insert_collection(
        &pool,
        depth_owner.id,
        parent_id,
        "too deep",
        None,
        CollectionVisibility::Private,
        None,
    )
    .await
    .is_err());

    let child_owner = crate::test_utils::insert_test_user(&pool).await;
    let parent = crate::db::insert_collection(
        &pool,
        child_owner.id,
        None,
        "parent",
        None,
        CollectionVisibility::Private,
        None,
    )
    .await
    .unwrap();
    for index in 0..MAX_COLLECTION_CHILDREN {
        crate::db::insert_collection(
            &pool,
            child_owner.id,
            Some(parent.id),
            &format!("child-{index}"),
            None,
            CollectionVisibility::Private,
            None,
        )
        .await
        .unwrap();
    }
    assert!(crate::db::insert_collection(
        &pool,
        child_owner.id,
        Some(parent.id),
        "one too many",
        None,
        CollectionVisibility::Private,
        None,
    )
    .await
    .is_err());
    assert_eq!(
        crate::db::get_child_collections(&pool, parent.id, child_owner.id)
            .await
            .unwrap()
            .len(),
        MAX_COLLECTION_CHILDREN
    );
}

#[tokio::test]
async fn moving_a_subtree_cannot_exceed_the_hierarchy_depth() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let owner = crate::test_utils::insert_test_user(&pool).await;
    let mut deep_parent = None;
    for depth in 1..MAX_COLLECTION_DEPTH {
        let collection = crate::db::insert_collection(
            &pool,
            owner.id,
            deep_parent,
            &format!("destination-{depth}"),
            None,
            CollectionVisibility::Private,
            None,
        )
        .await
        .unwrap();
        deep_parent = Some(collection.id);
    }
    let subtree = crate::db::insert_collection(
        &pool,
        owner.id,
        None,
        "subtree",
        None,
        CollectionVisibility::Private,
        None,
    )
    .await
    .unwrap();
    crate::db::insert_collection(
        &pool,
        owner.id,
        Some(subtree.id),
        "subtree child",
        None,
        CollectionVisibility::Private,
        None,
    )
    .await
    .unwrap();

    assert!(
        crate::db::move_collection(&pool, subtree.id, owner.id, deep_parent)
            .await
            .is_err()
    );
    assert_eq!(
        crate::db::get_collection_by_id_and_owner(&pool, subtree.id, owner.id)
            .await
            .unwrap()
            .unwrap()
            .parent_collection_id,
        None
    );
}

#[tokio::test]
async fn collection_membership_quota_rejects_without_partial_write() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let owner = crate::test_utils::insert_test_user(&pool).await;
    let collection = crate::db::insert_collection(
        &pool,
        owner.id,
        None,
        "membership quota",
        None,
        CollectionVisibility::Private,
        None,
    )
    .await
    .unwrap();
    let recipe_ids: Vec<uuid::Uuid> = sqlx::query_scalar(
        "INSERT INTO recipes (user_id, title) \
         SELECT $1, 'recipe-' || value::text FROM generate_series(0, $2) AS value \
         RETURNING id",
    )
    .bind(owner.id)
    .bind(i32::try_from(MAX_COLLECTION_MEMBERSHIPS).unwrap())
    .fetch_all(&pool)
    .await
    .unwrap();
    let retained_ids = &recipe_ids[..MAX_COLLECTION_MEMBERSHIPS];
    sqlx::query(
        "INSERT INTO collection_recipes (collection_id, recipe_id, sort_order) \
         SELECT $1, recipe_id, ordinality - 1 \
         FROM unnest($2::uuid[]) WITH ORDINALITY AS membership(recipe_id, ordinality)",
    )
    .bind(collection.id)
    .bind(retained_ids)
    .execute(&pool)
    .await
    .unwrap();

    assert!(crate::db::add_recipe_to_collection(
        &pool,
        collection.id,
        owner.id,
        recipe_ids[MAX_COLLECTION_MEMBERSHIPS],
    )
    .await
    .is_err());
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM collection_recipes WHERE collection_id = $1")
            .bind(collection.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, i64::try_from(MAX_COLLECTION_MEMBERSHIPS).unwrap());
}

#[tokio::test]
async fn active_draft_quota_is_atomic_and_user_scoped() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let saturated_user = crate::test_utils::insert_test_user(&pool).await;
    let other_user = crate::test_utils::insert_test_user(&pool).await;

    sqlx::query(
        "INSERT INTO recipe_drafts (user_id, snapshot) \
         SELECT $1, '{}'::jsonb FROM generate_series(1, $2)",
    )
    .bind(saturated_user.id)
    .bind(i32::try_from(MAX_ACTIVE_DRAFTS_PER_USER).unwrap())
    .execute(&pool)
    .await
    .unwrap();

    assert!(crate::db::create_new_recipe_draft(
        &pool,
        saturated_user.id,
        &draft_payload("blocked")
    )
    .await
    .is_err());
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM recipe_drafts WHERE user_id = $1")
        .bind(saturated_user.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, i64::try_from(MAX_ACTIVE_DRAFTS_PER_USER).unwrap());

    assert!(
        crate::db::create_new_recipe_draft(&pool, other_user.id, &draft_payload("allowed"))
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn immutable_version_quota_rejects_before_snapshot_write() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let owner = crate::test_utils::insert_test_user(&pool).await;
    let payload = draft_payload("Versioned recipe");
    let snapshot = payload.content.clone();
    let recipe = crate::db::create_published_recipe(&pool, owner.id, &payload, None)
        .await
        .unwrap();
    let snapshot_json = crate::db::diff::snapshot_to_json(&snapshot).unwrap();

    sqlx::query(
        "INSERT INTO recipe_versions (recipe_id, version_number, snapshot) \
         SELECT $1, value, $2 FROM generate_series(2, $3) AS value",
    )
    .bind(recipe.id)
    .bind(snapshot_json)
    .bind(i32::try_from(MAX_VERSIONS_PER_RECIPE).unwrap())
    .execute(&pool)
    .await
    .unwrap();

    assert!(crate::db::publish_snapshot(
        &pool,
        recipe.id,
        owner.id,
        i32::try_from(MAX_VERSIONS_PER_RECIPE).unwrap(),
        &snapshot,
        None,
    )
    .await
    .is_err());
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM recipe_versions WHERE recipe_id = $1")
            .bind(recipe.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, i64::try_from(MAX_VERSIONS_PER_RECIPE).unwrap());
}

#[tokio::test]
async fn immutable_version_user_quota_is_independent_of_recipe_quota() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let owner = crate::test_utils::insert_test_user(&pool).await;
    let other_owner = crate::test_utils::insert_test_user(&pool).await;
    let payload = draft_payload("Version storage quota");
    let snapshot_json = crate::db::diff::snapshot_to_json(&payload.content).unwrap();
    let recipe_count = MAX_VERSIONS_PER_USER.div_ceil(MAX_VERSIONS_PER_RECIPE);
    let recipe_ids: Vec<uuid::Uuid> = sqlx::query_scalar(
        "INSERT INTO recipes (user_id, title) \
         SELECT $1, 'quota-recipe-' || value::text FROM generate_series(1, $2) AS value \
         RETURNING id",
    )
    .bind(owner.id)
    .bind(i32::try_from(recipe_count).unwrap())
    .fetch_all(&pool)
    .await
    .unwrap();
    for recipe_id in recipe_ids {
        sqlx::query(
            "INSERT INTO recipe_versions (recipe_id, version_number, snapshot) \
             SELECT $1, value, $2 FROM generate_series(1, $3) AS value",
        )
        .bind(recipe_id)
        .bind(snapshot_json.clone())
        .bind(i32::try_from(MAX_VERSIONS_PER_RECIPE).unwrap())
        .execute(&pool)
        .await
        .unwrap();
    }

    let recipe_count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM recipes WHERE user_id = $1")
            .bind(owner.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(matches!(
        crate::db::create_published_recipe(&pool, owner.id, &payload, None).await,
        Err(crate::db::DbError::ResourceLimitExceeded(_))
    ));
    let recipe_count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM recipes WHERE user_id = $1")
            .bind(owner.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(recipe_count_after, recipe_count_before);

    assert!(
        crate::db::create_published_recipe(&pool, other_owner.id, &payload, None)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn public_tag_catalog_has_a_server_side_result_ceiling() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let owner = crate::test_utils::insert_test_user(&pool).await;
    let recipe_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO recipes (user_id, title, visibility) VALUES ($1, 'tag catalog', 'public') \
         RETURNING id",
    )
    .bind(owner.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO recipe_tags (recipe_id, tag) \
         SELECT $1, 'tag-' || lpad(value::text, 5, '0') FROM generate_series(0, $2) AS value",
    )
    .bind(recipe_id)
    .bind(i32::try_from(MAX_PUBLIC_TAGS).unwrap())
    .execute(&pool)
    .await
    .unwrap();

    let tags = crate::db::get_distinct_public_tags(&pool).await.unwrap();
    assert_eq!(tags.len(), MAX_PUBLIC_TAGS);
    assert_eq!(tags.first().map(String::as_str), Some("tag-00000"));
    let expected_last = format!("tag-{:05}", MAX_PUBLIC_TAGS - 1);
    assert_eq!(tags.last(), Some(&expected_last));
}

#[tokio::test]
async fn collection_image_metadata_limit_rejects_before_persistence() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let owner = crate::test_utils::insert_test_user(&pool).await;
    let mut edit = crate::types::RecipeImageEdit::default_rect_16x9();
    edit.name = "e".repeat(crate::security::MAX_IMAGE_EDIT_NAME_CHARS + 1);
    let image = crate::types::RecipeImage {
        url: "https://media.example.test/collection.webp".to_string(),
        thumb_url: Some("https://media.example.test/collection-thumb.webp".to_string()),
        file_name: Some("cover.webp".to_string()),
        file_size: Some(1),
        edits: vec![edit],
    };

    assert!(crate::db::insert_collection(
        &pool,
        owner.id,
        None,
        "oversized metadata",
        None,
        CollectionVisibility::Private,
        Some(&image),
    )
    .await
    .is_err());
    let collection_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM collections WHERE owner_id = $1")
            .bind(owner.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(collection_count, 0);
}
