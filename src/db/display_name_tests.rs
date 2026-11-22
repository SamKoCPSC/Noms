use super::*;
use crate::test_utils;

async fn user(pool: &PgPool, name: &str) -> User {
    let suffix = test_utils::uid();
    insert_user(
        pool,
        &format!("name_{suffix}"),
        name,
        &format!("{suffix}@example.test"),
        None,
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn display_name_constraint_rejects_direct_sql_case_and_space_duplicates() {
    let (_db, pool) = test_utils::setup_test_db().await;
    let _owner = user(&pool, "Recipe Cook").await;
    for name in ["Recipe Cook", "recipe cook", " RECIPE COOK "] {
        let suffix = test_utils::uid();
        let error =
            sqlx::query("INSERT INTO users (username, display_name, email) VALUES ($1, $2, $3)")
                .bind(format!("raw_{suffix}"))
                .bind(name)
                .bind(format!("{suffix}@example.test"))
                .execute(&pool)
                .await
                .unwrap_err();
        let sqlx::Error::Database(error) = error else {
            panic!("expected database conflict")
        };
        assert!(error.is_unique_violation());
        assert_eq!(error.constraint(), Some("users_display_name_unique"));
    }
    let error = insert_user(
        &pool,
        "typed_duplicate",
        "recipe cook",
        "typed@example.test",
        None,
    )
    .await
    .unwrap_err();
    assert!(matches!(error, DbError::DisplayNameTaken));
}

#[tokio::test]
async fn display_name_conflict_rolls_back_entire_profile_save() {
    let (_db, pool) = test_utils::setup_test_db().await;
    let owner = user(&pool, "First Cook").await;
    let _other = user(&pool, "Second Cook").await;
    let error = update_user_profile_and_username(
        &pool,
        owner.id,
        "second COOK",
        Some("must not save"),
        Some("changed_username"),
    )
    .await
    .unwrap_err();
    assert!(matches!(error, DbError::DisplayNameTaken));
    let unchanged = get_user_by_id(&pool, owner.id).await.unwrap().unwrap();
    assert_eq!(unchanged.username, owner.username);
    assert_eq!(unchanged.display_name, owner.display_name);
    assert_eq!(unchanged.bio, owner.bio);
    assert_eq!(unchanged.updated_at, owner.updated_at);

    let updated = update_user_profile_and_username(
        &pool,
        owner.id,
        "  New Cook  ",
        Some("saved"),
        Some("new_username"),
    )
    .await
    .unwrap();
    assert_eq!(updated.display_name, "New Cook");
    assert_eq!(updated.username, "new_username");
    assert_eq!(updated.bio.as_deref(), Some("saved"));
}

#[tokio::test]
async fn username_conflict_also_rolls_back_display_name_and_bio() {
    let (_db, pool) = test_utils::setup_test_db().await;
    let owner = user(&pool, "Original Cook").await;
    let other = user(&pool, "Other Cook").await;
    let error = update_user_profile_and_username(
        &pool,
        owner.id,
        "Unsaved Cook",
        Some("unsaved"),
        Some(&other.username),
    )
    .await
    .unwrap_err();
    assert!(matches!(error, DbError::UsernameTaken));
    let unchanged = get_user_by_id(&pool, owner.id).await.unwrap().unwrap();
    assert_eq!(unchanged.username, owner.username);
    assert_eq!(unchanged.display_name, owner.display_name);
    assert_eq!(unchanged.bio, owner.bio);
}

#[tokio::test]
async fn display_name_can_keep_own_name_or_change_its_capitalization() {
    let (_db, pool) = test_utils::setup_test_db().await;
    let owner = user(&pool, "My Name").await;
    let saved = update_user_profile(&pool, owner.id, "My Name", Some("bio only"))
        .await
        .unwrap();
    assert_eq!(saved.display_name, "My Name");
    let saved = update_user_profile(&pool, owner.id, "  MY name  ", None)
        .await
        .unwrap();
    assert_eq!(saved.display_name, "MY name");
}

#[tokio::test]
async fn concurrent_display_name_claims_have_exactly_one_winner() {
    let (_db, pool) = test_utils::setup_test_db().await;
    let first = user(&pool, "First Claimant").await;
    let second = user(&pool, "Second Claimant").await;
    let (a, b) = tokio::join!(
        update_user_profile(&pool, first.id, "Shared Cook", None),
        update_user_profile(&pool, second.id, " shared cook ", None),
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert!(
        matches!(a, Err(DbError::DisplayNameTaken)) || matches!(b, Err(DbError::DisplayNameTaken))
    );
}
