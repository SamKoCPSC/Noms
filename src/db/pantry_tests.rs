use super::pantry::*;
use crate::pantry::*;
use sqlx::PgPool;
use uuid::Uuid;
async fn user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    let suffix = id.simple().to_string();
    sqlx::query(
        "INSERT INTO users(id,username,display_name,email) VALUES($1,$2,'Pantry tester',$3)",
    )
    .bind(id)
    .bind(&suffix[..24])
    .bind(format!("{suffix}@example.test"))
    .execute(pool)
    .await
    .unwrap();
    id
}
fn stock(name: &str, amount: &str, unit: &str) -> PantryStock {
    PantryStock {
        fields: PantryFields {
            name: name.into(),
            amount: amount.into(),
            unit: unit.into(),
        },
        available: true,
    }
}
fn request(name: &str) -> PantryCreate {
    PantryCreate {
        request_id: Uuid::new_v4(),
        name: name.into(),
        ingredients: vec![
            stock("Rice", "2", "cup"),
            stock("Olive oil", "", ""),
            stock("Salt", "0", "g"),
        ],
        equipment: vec![stock("Blender", "", "")],
        ..Default::default()
    }
}
#[tokio::test]
async fn pantry_persists_unknown_zero_exact_names_and_owner_isolation() {
    let (_db, pool) = crate::test_utils::setup_test_db().await;
    let a = user(&pool).await;
    let b = user(&pool).await;
    let req = request("Home");
    let p = create(&pool, a, req.clone()).await.unwrap();
    assert_eq!(p.entries.len(), 4);
    assert_eq!(create(&pool, a, req.clone()).await.unwrap().id, p.id);
    assert!(matches!(
        create(
            &pool,
            a,
            PantryCreate {
                name: "Changed".into(),
                ..req
            }
        )
        .await,
        Err(PantryError::RetryMismatch)
    ));
    let summary = summaries(&pool, a).await.unwrap().remove(0);
    assert_eq!(summary.in_stock_count, 2);
    assert_eq!(summary.ingredient_count, 3);
    assert_eq!(summary.available_equipment_count, 1);
    assert!(summaries(&pool, b).await.unwrap().is_empty());
    assert!(matches!(
        get(&pool, b, p.id).await,
        Err(PantryError::Missing)
    ));
    let item = &p.entries[0];
    assert!(matches!(
        change(
            &pool,
            b,
            p.id,
            item.kind,
            item.id,
            0,
            PantryChange::Availability(false)
        )
        .await,
        Err(PantryError::Missing)
    ));
    assert!(matches!(
        add(
            &pool,
            b,
            p.id,
            PantryKind::Ingredient,
            Uuid::new_v4(),
            stock("Stolen", "", "")
        )
        .await,
        Err(PantryError::Missing)
    ));
    assert!(matches!(
        edit_profile(&pool, b, p.id, 0, "Stolen".into(), "".into()).await,
        Err(PantryError::Missing)
    ));
    assert!(matches!(
        delete(&pool, b, p.id, 0).await,
        Err(PantryError::Missing)
    ));
    let empty = create(
        &pool,
        a,
        PantryCreate {
            ingredients: vec![],
            equipment: vec![],
            ..request("Cabin")
        },
    )
    .await
    .unwrap();
    assert!(empty.entries.is_empty());
    assert!(matches!(
        change(
            &pool,
            a,
            empty.id,
            item.kind,
            item.id,
            0,
            PantryChange::Availability(false)
        )
        .await,
        Err(PantryError::Missing)
    ));
    assert!(matches!(
        remove(&pool, a, empty.id, item.kind, item.id, 0).await,
        Err(PantryError::Missing)
    ));
    assert!(matches!(
        change(
            &pool,
            a,
            p.id,
            PantryKind::Equipment,
            item.id,
            0,
            PantryChange::Availability(false)
        )
        .await,
        Err(PantryError::Missing)
    ));
    for s in [
        stock("Rice", "1", "cups"),
        stock("rice", "200", "g"),
        stock(" Rice ", "1/2", "cup"),
        stock("Rice", "3", "cup"),
    ] {
        add(&pool, a, p.id, PantryKind::Ingredient, Uuid::new_v4(), s)
            .await
            .unwrap();
    }
    let saved = get(&pool, a, p.id).await.unwrap();
    assert_eq!(saved.entries.len(), 8);
    assert_eq!(
        saved
            .entries
            .iter()
            .filter(|x| x.stock.fields.name == "Rice" && x.stock.fields.unit == "cup")
            .count(),
        2
    );
}
#[tokio::test]
async fn pantry_edits_preserve_availability_and_duplicate_is_independent() {
    let (_db, pool) = crate::test_utils::setup_test_db().await;
    let a = user(&pool).await;
    let p = create(&pool, a, request("Home")).await.unwrap();
    let rice = &p.entries[0];
    change(
        &pool,
        a,
        p.id,
        rice.kind,
        rice.id,
        0,
        PantryChange::Availability(false),
    )
    .await
    .unwrap();
    let mut fields = rice.stock.fields.clone();
    fields.name = "Brown rice".into();
    fields.amount = "500".into();
    fields.unit = "g".into();
    change(
        &pool,
        a,
        p.id,
        rice.kind,
        rice.id,
        1,
        PantryChange::Fields(fields.clone()),
    )
    .await
    .unwrap();
    let source = get(&pool, a, p.id).await.unwrap();
    let row = source.entries.iter().find(|x| x.id == rice.id).unwrap();
    assert!(!row.stock.available);
    assert_eq!(row.stock.fields, fields);
    let copy_request = PantryCreate {
        request_id: Uuid::new_v4(),
        name: "Home copy".into(),
        copy: Some(PantryCopy {
            id: p.id,
            expected_revision: source.revision,
        }),
        ..Default::default()
    };
    let copy = create(&pool, a, copy_request.clone()).await.unwrap();
    assert_eq!(copy.entries.len(), 4);
    assert!(copy
        .entries
        .iter()
        .all(|e| source.entries.iter().all(|s| s.id != e.id)));
    assert!(!copy.entries[0].stock.available);
    change(
        &pool,
        a,
        p.id,
        rice.kind,
        rice.id,
        2,
        PantryChange::Availability(true),
    )
    .await
    .unwrap();
    assert_eq!(create(&pool, a, copy_request).await.unwrap().id, copy.id);
    assert!(
        !get(&pool, a, copy.id).await.unwrap().entries[0]
            .stock
            .available
    );
    let tool = copy
        .entries
        .iter()
        .find(|x| x.kind == PantryKind::Equipment)
        .unwrap();
    remove(&pool, a, copy.id, tool.kind, tool.id, 0)
        .await
        .unwrap();
    assert_eq!(get(&pool, a, p.id).await.unwrap().entries.len(), 4);
    let latest = get(&pool, a, copy.id).await.unwrap();
    delete(&pool, a, copy.id, latest.revision).await.unwrap();
    let rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM pantry_ingredients WHERE profile_id=$1")
            .bind(copy.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(rows, 0);
}
#[tokio::test]
async fn pantry_retries_concurrency_and_stale_writes() {
    let (_db, pool) = crate::test_utils::setup_test_db().await;
    let a = user(&pool).await;
    let p = create(&pool, a, request("Home")).await.unwrap();
    let key = Uuid::new_v4();
    let s = stock("Flour", "1 1/2", "kg");
    let (a1, a2) = tokio::join!(
        add(&pool, a, p.id, PantryKind::Ingredient, key, s.clone()),
        add(&pool, a, p.id, PantryKind::Ingredient, key, s.clone())
    );
    a1.unwrap();
    a2.unwrap();
    assert_eq!(get(&pool, a, p.id).await.unwrap().entries.len(), 5);
    assert!(matches!(
        add(
            &pool,
            a,
            p.id,
            PantryKind::Ingredient,
            key,
            stock("Flour", "2", "kg")
        )
        .await,
        Err(PantryError::RetryMismatch)
    ));
    let first = &p.entries[0];
    let second = &p.entries[1];
    let (r1, r2) = tokio::join!(
        change(
            &pool,
            a,
            p.id,
            first.kind,
            first.id,
            0,
            PantryChange::Availability(false)
        ),
        change(
            &pool,
            a,
            p.id,
            second.kind,
            second.id,
            0,
            PantryChange::Availability(false)
        )
    );
    r1.unwrap();
    r2.unwrap();
    change(
        &pool,
        a,
        p.id,
        first.kind,
        first.id,
        0,
        PantryChange::Availability(false),
    )
    .await
    .unwrap();
    assert!(matches!(
        change(
            &pool,
            a,
            p.id,
            first.kind,
            first.id,
            0,
            PantryChange::Availability(true)
        )
        .await,
        Err(PantryError::Conflict)
    ));
    assert!(matches!(
        remove(&pool, a, p.id, first.kind, first.id, 0).await,
        Err(PantryError::Conflict)
    ));
    assert!(matches!(
        delete(&pool, a, p.id, 0).await,
        Err(PantryError::Conflict)
    ));
    assert!(matches!(
        edit_profile(&pool, a, p.id, 0, "Changed".into(), "".into()).await,
        Err(PantryError::Conflict)
    ));
}
#[tokio::test]
async fn pantry_invalid_requests_roll_back_and_grocery_lists_survive() {
    let (_db, pool) = crate::test_utils::setup_test_db().await;
    let a = user(&pool).await;
    let b = user(&pool).await;
    let p = create(&pool, a, request("Home")).await.unwrap();
    for invalid in [
        stock("Negative", "-1", "cup"),
        stock("Overflow", "99999999999999999", "g"),
        stock("NaN", "NaN", "g"),
        stock("Bad fraction", "1/0", "g"),
    ] {
        assert!(add(
            &pool,
            a,
            p.id,
            PantryKind::Ingredient,
            Uuid::new_v4(),
            invalid
        )
        .await
        .is_err());
    }
    assert_eq!(get(&pool, a, p.id).await.unwrap().entries.len(), 4);
    let foreign = create(&pool, b, request("Other")).await.unwrap();
    let before = summaries(&pool, a).await.unwrap().len();
    assert!(matches!(
        create(
            &pool,
            a,
            PantryCreate {
                request_id: Uuid::new_v4(),
                name: "Stolen copy".into(),
                copy: Some(PantryCopy {
                    id: foreign.id,
                    expected_revision: 0
                }),
                ..Default::default()
            }
        )
        .await,
        Err(PantryError::Missing)
    ));
    assert!(matches!(
        create(
            &pool,
            a,
            PantryCreate {
                request_id: Uuid::new_v4(),
                name: "Stale copy".into(),
                copy: Some(PantryCopy {
                    id: p.id,
                    expected_revision: 999
                }),
                ..Default::default()
            }
        )
        .await,
        Err(PantryError::Conflict)
    ));
    assert_eq!(summaries(&pool, a).await.unwrap().len(), before);
    let list = crate::db::grocery::create(
        &pool,
        a,
        crate::grocery::GroceryCreate {
            request_id: Uuid::new_v4(),
            name: "Essentials".into(),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    delete(&pool, a, p.id, 0).await.unwrap();
    assert_eq!(
        crate::db::grocery::get(&pool, a, list.id)
            .await
            .unwrap()
            .name,
        "Essentials"
    );
}
#[test]
fn pantry_schema_fixture_matches_declarative_schema() {
    assert!(include_str!("../../migrations/schema.sql")
        .contains(include_str!("../../migrations/pantry.sql")));
}
