use super::*;
use crate::db;
use crate::db::diff::RecipeDraftPayload;
use crate::test_utils;

fn payload(title: &str) -> RecipeDraftPayload {
    RecipeDraftPayload {
        content: test_utils::test_recipe_snapshot(title),
        initial_visibility: Some("private".to_string()),
    }
}

#[tokio::test]
async fn names_ignore_case_and_whitespace_per_owner_across_visibility() {
    let (_database, pool) = test_utils::setup_test_db().await;
    let owner = test_utils::insert_test_user(&pool).await;
    let other = test_utils::insert_test_user(&pool).await;
    db::create_published_recipe(&pool, owner.id, &payload("Chicken curry"), None)
        .await
        .unwrap();
    for visibility in ["private", "unlisted", "public"] {
        let mut duplicate = payload("  CHICKEN \t curry  ");
        duplicate.initial_visibility = Some(visibility.to_string());
        assert!(matches!(
            db::create_published_recipe(&pool, owner.id, &duplicate, None).await,
            Err(DbError::RecipeNameTaken)
        ));
    }
    db::create_published_recipe(&pool, other.id, &payload("chicken curry"), None)
        .await
        .unwrap();
    db::create_published_recipe(&pool, owner.id, &payload("Chicken soup"), None)
        .await
        .unwrap();
}

#[tokio::test]
async fn conflicting_drafts_survive_publication_and_can_be_renamed() {
    let (_database, pool) = test_utils::setup_test_db().await;
    let owner = test_utils::insert_test_user(&pool).await;
    db::create_published_recipe(&pool, owner.id, &payload("Soup"), None)
        .await
        .unwrap();
    let draft = db::create_new_recipe_draft(&pool, owner.id, &payload(" SOUP "))
        .await
        .unwrap();
    db::create_new_recipe_draft(&pool, owner.id, &payload(" SOUP "))
        .await
        .unwrap();
    assert!(matches!(
        db::publish_recipe_draft(&pool, draft.id, owner.id, None).await,
        Err(DbError::RecipeNameTaken)
    ));
    let retained = db::get_recipe_draft(&pool, draft.id, owner.id)
        .await
        .unwrap();
    assert_eq!(retained.revision, draft.revision);
    let revised = db::update_recipe_draft(
        &pool,
        draft.id,
        owner.id,
        retained.revision,
        &payload("Lemon soup"),
    )
    .await
    .unwrap();
    let published = db::publish_recipe_draft(&pool, revised.id, owner.id, None)
        .await
        .unwrap();
    assert_eq!(published.title, "Lemon soup");
}

#[tokio::test]
async fn own_name_is_allowed_and_conflicting_restore_rolls_back_history() {
    let (_database, pool) = test_utils::setup_test_db().await;
    let owner = test_utils::insert_test_user(&pool).await;
    let recipe = db::create_published_recipe(&pool, owner.id, &payload("Soup"), None)
        .await
        .unwrap();
    db::publish_snapshot(
        &pool,
        recipe.id,
        owner.id,
        1,
        &payload("Broth").content,
        None,
    )
    .await
    .unwrap();
    db::create_published_recipe(&pool, owner.id, &payload("Soup"), None)
        .await
        .unwrap();
    assert!(matches!(
        db::restore_recipe_version(&pool, recipe.id, owner.id, 1, 2).await,
        Err(DbError::RecipeNameTaken)
    ));
    let latest = db::get_latest_recipe_version(&pool, recipe.id)
        .await
        .unwrap();
    assert_eq!(latest.version_number, 2);
    let actual = db::get_recipe_by_id_and_owner(&pool, recipe.id, owner.id)
        .await
        .unwrap();
    assert_eq!(actual.title, "Broth");
    let renamed = db::publish_snapshot(
        &pool,
        recipe.id,
        owner.id,
        2,
        &payload("BROTH").content,
        None,
    )
    .await
    .unwrap();
    assert_eq!(renamed.title, "BROTH");
}

#[tokio::test]
async fn fork_draft_can_be_renamed_before_publishing_for_the_same_owner() {
    let (_database, pool) = test_utils::setup_test_db().await;
    let owner = test_utils::insert_test_user(&pool).await;
    let source = db::create_published_recipe(&pool, owner.id, &payload("Curry"), None)
        .await
        .unwrap();
    let fork = db::create_fork_draft(&pool, source.id, owner.id, None)
        .await
        .unwrap();
    assert!(matches!(
        db::publish_recipe_draft(&pool, fork.id, owner.id, None).await,
        Err(DbError::RecipeNameTaken)
    ));
    let mut fork_payload = fork.decoded_snapshot().unwrap();
    fork_payload.content.title = "Slow cooker curry".to_string();
    db::update_recipe_draft(&pool, fork.id, owner.id, fork.revision, &fork_payload)
        .await
        .unwrap();
    let recipe = db::publish_recipe_draft(&pool, fork.id, owner.id, None)
        .await
        .unwrap();
    assert_eq!(recipe.title, "Slow cooker curry");
    assert!(db::get_fork_relationship(&pool, recipe.id)
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn concurrent_direct_writes_are_guarded_by_the_database_index() {
    let (_database, pool) = test_utils::setup_test_db().await;
    let owner = test_utils::insert_test_user(&pool).await;
    let first = sqlx::query("INSERT INTO recipes(user_id,title) VALUES($1,$2)")
        .bind(owner.id)
        .bind("Chicken curry")
        .execute(&pool);
    let second = sqlx::query("INSERT INTO recipes(user_id,title) VALUES($1,$2)")
        .bind(owner.id)
        .bind("  CHICKEN   CURRY ")
        .execute(&pool);
    let (first, second) = tokio::join!(first, second);
    assert_ne!(first.is_ok(), second.is_ok());
    let error = first.err().or_else(|| second.err()).unwrap();
    assert!(matches!(map_write_error(error), DbError::RecipeNameTaken));
}

#[tokio::test]
async fn availability_is_owner_scoped_and_excludes_only_an_owned_recipe() {
    let (_database, pool) = test_utils::setup_test_db().await;
    let owner = test_utils::insert_test_user(&pool).await;
    let other = test_utils::insert_test_user(&pool).await;
    let recipe = db::create_published_recipe(&pool, owner.id, &payload("Curry"), None)
        .await
        .unwrap();
    assert!(!recipe_name_available(&pool, owner.id, " CURRY ", None).await.unwrap());
    assert!(recipe_name_available(&pool, owner.id, "curry", Some(recipe.id)).await.unwrap());
    assert!(recipe_name_available(&pool, other.id, "curry", None).await.unwrap());
    assert!(matches!(
        recipe_name_available(&pool, other.id, "curry", Some(recipe.id)).await,
        Err(DbError::RecipeNotFound(_))
    ));
}
