undefined
#[tokio::test]
async fn list_name_conflicts_preserve_items_revisions_and_retry_identity() {
    let (_db, pool) = crate::test_utils::setup_test_db().await;
    let owner = user(&pool).await;
    let req = request("Weekly shop");
    let saved = create(&pool, owner, req.clone()).await.unwrap();
    assert_eq!(create(&pool, owner, req).await.unwrap().id, saved.id);
    let mut duplicate = request(" WEEKLY  shop ");
    assert!(matches!(create(&pool, owner, duplicate.clone()).await, Err(GroceryError::NameTaken)));
    assert_eq!(summaries(&pool, owner).await.unwrap().len(), 1);
    duplicate.name = "Weekend shop".into();
    let second = create(&pool, owner, duplicate).await.unwrap();
    assert!(matches!(
        rename(&pool, owner, second.id, 0, "weekly shop".into()).await,
        Err(GroceryError::NameTaken)
    ));
    let unchanged = get(&pool, owner, second.id).await.unwrap();
    assert_eq!(unchanged.name, "Weekend shop");
    assert_eq!(unchanged.revision, 0);
    assert_eq!(unchanged.items.len(), 2);
    rename(&pool, owner, second.id, 0, "WEEKEND shop".into()).await.unwrap();
}
