undefined
#[tokio::test]
async fn pantry_name_conflicts_preserve_inventory_revisions_and_copy_retries() {
    let (_db, pool) = crate::test_utils::setup_test_db().await;
    let owner = user(&pool).await;
    let req = request("Home pantry");
    let saved = create(&pool, owner, req.clone()).await.unwrap();
    assert_eq!(create(&pool, owner, req).await.unwrap().id, saved.id);
    assert!(matches!(
        create(&pool, owner, request(" HOME  pantry ")).await,
        Err(PantryError::NameTaken)
    ));
    let mut copy = PantryCreate {
        request_id: Uuid::new_v4(),
        name: "home pantry".into(),
        copy: Some(PantryCopy { id: saved.id, expected_revision: saved.revision }),
        ..Default::default()
    };
    assert!(matches!(create(&pool, owner, copy.clone()).await, Err(PantryError::NameTaken)));
    assert_eq!(summaries(&pool, owner).await.unwrap().len(), 1);
    copy.name = "Cabin pantry".into();
    let second = create(&pool, owner, copy).await.unwrap();
    assert!(matches!(
        edit_profile(&pool, owner, second.id, 0, "home pantry".into(), "Changed note".into()).await,
        Err(PantryError::NameTaken)
    ));
    let unchanged = get(&pool, owner, second.id).await.unwrap();
    assert_eq!(unchanged.name, "Cabin pantry");
    assert_eq!(unchanged.note, "");
    assert_eq!(unchanged.revision, 0);
    assert_eq!(unchanged.entries.len(), saved.entries.len());
    edit_profile(&pool, owner, second.id, 0, "CABIN pantry".into(), "".into()).await.unwrap();
}
