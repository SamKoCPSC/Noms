use super::grocery::*;
use crate::grocery::*;
use sqlx::PgPool;
use uuid::Uuid;

async fn user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    let suffix = id.simple().to_string();
    sqlx::query(
        "INSERT INTO users(id,username,display_name,email) VALUES($1,$2,'Grocery tester',$3)",
    )
    .bind(id)
    .bind(&suffix[..24])
    .bind(format!("{suffix}@example.test"))
    .execute(pool)
    .await
    .unwrap();
    id
}
fn fields(name: &str, amount: &str, unit: &str) -> ItemFields {
    ItemFields {
        name: name.into(),
        amount: amount.into(),
        unit: unit.into(),
    }
}
fn request(name: &str) -> GroceryCreate {
    GroceryCreate {
        request_id: Uuid::new_v4(),
        name: name.into(),
        note: String::new(),
        recipes: vec![],
        manual_items: vec![
            fields("Dish soap", "1", "bottle"),
            fields("Coffee", "250", "g"),
        ],
    }
}
async fn recipe(pool: &PgPool, owner: Uuid, visibility: &str, amount: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO recipes(id,user_id,title,servings,ingredients,visibility) VALUES($1,$2,'Dinner',2,$3,$4)").bind(id).bind(owner).bind(serde_json::json!([{"name":"Rice","amount":amount,"unit":"cup"},{"name":"Garnish (optional)","amount":"to taste","unit":""}])).bind(visibility).execute(pool).await.unwrap();
    id
}
async fn selection(pool: &PgPool, owner: Uuid, id: Uuid, target: Option<i32>) -> RecipeSelection {
    let mut conn = pool.acquire().await.unwrap();
    let preview = preview_source(&mut conn, owner, id).await.unwrap();
    RecipeSelection {
        recipe_id: id,
        target_servings: target,
        expected_revision: preview.source.revision,
    }
}

#[tokio::test]
async fn manual_lists_persist_isolate_owners_and_retry_without_duplicates() {
    let (_db, pool) = crate::test_utils::setup_test_db().await;
    let a = user(&pool).await;
    let b = user(&pool).await;
    let req = request("Essentials");
    let saved = create(&pool, a, req.clone()).await.unwrap();
    assert!(saved.sources.is_empty());
    assert_eq!(saved.items.len(), 2);
    let retry = create(&pool, a, req.clone()).await.unwrap();
    assert_eq!(retry.id, saved.id);
    let mut altered = req;
    altered.name = "Another list".into();
    assert!(matches!(
        create(&pool, a, altered).await,
        Err(GroceryError::RetryMismatch)
    ));
    assert_eq!(summaries(&pool, a).await.unwrap().len(), 1);
    assert!(summaries(&pool, b).await.unwrap().is_empty());
    assert!(matches!(
        get(&pool, b, saved.id).await,
        Err(GroceryError::Missing)
    ));
    let item = &saved.items[0];
    assert!(matches!(
        change_item(
            &pool,
            b,
            saved.id,
            item.id,
            0,
            GroceryItemChange::Checked(true)
        )
        .await,
        Err(GroceryError::Missing)
    ));
    assert!(matches!(
        rename(&pool, b, saved.id, 0, "Steal".into()).await,
        Err(GroceryError::Missing)
    ));
    assert!(matches!(
        delete(&pool, b, saved.id, 0).await,
        Err(GroceryError::Missing)
    ));
    let empty = create(
        &pool,
        a,
        GroceryCreate {
            manual_items: vec![],
            ..request("Empty")
        },
    )
    .await
    .unwrap();
    assert!(empty.items.is_empty());
    assert!(matches!(
        change_item(
            &pool,
            a,
            empty.id,
            item.id,
            0,
            GroceryItemChange::Checked(true)
        )
        .await,
        Err(GroceryError::Missing)
    ));
    let add_key = Uuid::new_v4();
    add_item(&pool, a, empty.id, add_key, fields("Apples", "4", ""))
        .await
        .unwrap();
    add_item(&pool, a, empty.id, add_key, fields("Apples", "4", ""))
        .await
        .unwrap();
    assert_eq!(get(&pool, a, empty.id).await.unwrap().items.len(), 1);
    assert!(matches!(
        add_item(&pool, a, empty.id, add_key, fields("Pears", "4", "")).await,
        Err(GroceryError::RetryMismatch)
    ));
    let loaded = get(&pool, a, empty.id).await.unwrap();
    delete(&pool, a, empty.id, loaded.revision).await.unwrap();
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM grocery_list_items WHERE list_id=$1")
            .bind(empty.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(remaining, 0);
}

#[tokio::test]
async fn accessible_recipe_snapshots_scale_and_remain_independent_after_full_edits() {
    let (_db, pool) = crate::test_utils::setup_test_db().await;
    let a = user(&pool).await;
    let b = user(&pool).await;
    let own = recipe(&pool, a, "private", "1").await;
    let public = recipe(&pool, b, "public", "2").await;
    let unlisted = recipe(&pool, b, "unlisted", "3").await;
    let selections = vec![
        selection(&pool, a, own, Some(4)).await,
        selection(&pool, a, public, None).await,
        selection(&pool, a, unlisted, None).await,
    ];
    let saved = create(
        &pool,
        a,
        GroceryCreate {
            recipes: selections,
            ..request("Dinner shop")
        },
    )
    .await
    .unwrap();
    assert_eq!(saved.sources.len(), 3);
    let rice = saved
        .items
        .iter()
        .find(|i| i.fields.name == "Rice")
        .unwrap();
    assert_eq!(rice.fields.amount, "7");
    assert_eq!(rice.fields.unit, "cup");
    assert_eq!(rice.contributions.len(), 3);
    assert_eq!(
        saved
            .items
            .iter()
            .filter(|i| i.fields.name == "Garnish (optional)")
            .count(),
        3
    );
    change_item(
        &pool,
        a,
        saved.id,
        rice.id,
        0,
        GroceryItemChange::Checked(true),
    )
    .await
    .unwrap();
    change_item(
        &pool,
        a,
        saved.id,
        rice.id,
        1,
        GroceryItemChange::Fields(fields("Brown rice", "500", "g")),
    )
    .await
    .unwrap();
    sqlx::query("DELETE FROM recipes WHERE id=$1")
        .bind(own)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE recipes SET visibility='private',ingredients='[]' WHERE id IN ($1,$2)")
        .bind(public)
        .bind(unlisted)
        .execute(&pool)
        .await
        .unwrap();
    let loaded = get(&pool, a, saved.id).await.unwrap();
    let edited = loaded.items.iter().find(|i| i.id == rice.id).unwrap();
    assert!(edited.edited());
    assert!(edited.checked);
    assert_eq!(edited.original.as_ref().unwrap().amount, "7");
    assert_eq!(edited.fields.unit, "g");
    assert_eq!(edited.contributions[0].original.unit, "cup");
    assert_eq!(edited.contributions.len(), 3);
    remove_item(&pool, a, saved.id, edited.id, edited.revision)
        .await
        .unwrap();
    assert!(!get(&pool, a, saved.id)
        .await
        .unwrap()
        .items
        .iter()
        .any(|i| i.id == edited.id));
}

#[tokio::test]
async fn invalid_or_changed_sources_roll_back_the_whole_creation() {
    let (_db, pool) = crate::test_utils::setup_test_db().await;
    let a = user(&pool).await;
    let b = user(&pool).await;
    let public = recipe(&pool, b, "public", "1").await;
    let private = recipe(&pool, b, "private", "2").await;
    let chosen = selection(&pool, a, public, None).await;
    let inaccessible = RecipeSelection {
        recipe_id: private,
        target_servings: None,
        expected_revision: "a".repeat(64),
    };
    assert!(create(
        &pool,
        a,
        GroceryCreate {
            recipes: vec![chosen.clone(), inaccessible],
            ..request("Do not partially save")
        }
    )
    .await
    .is_err());
    assert!(summaries(&pool, a).await.unwrap().is_empty());
    sqlx::query("UPDATE recipes SET title='Changed dinner' WHERE id=$1")
        .bind(public)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        create(
            &pool,
            a,
            GroceryCreate {
                recipes: vec![chosen],
                ..request("Changed")
            }
        )
        .await,
        Err(GroceryError::SourceChanged)
    ));
    assert!(summaries(&pool, a).await.unwrap().is_empty());
    let mut bad = request("  ");
    assert!(create(&pool, a, bad.clone()).await.is_err());
    bad.name = "Valid".into();
    bad.manual_items[0].unit = "a".repeat(65);
    assert!(create(&pool, a, bad).await.is_err());
}

#[tokio::test]
async fn revisions_prevent_lost_edits_without_conflicting_independent_checkoffs() {
    let (_db, pool) = crate::test_utils::setup_test_db().await;
    let owner = user(&pool).await;
    let saved = create(&pool, owner, request("Shared devices"))
        .await
        .unwrap();
    let first = &saved.items[0];
    let second = &saved.items[1];
    let (a, b) = tokio::join!(
        change_item(
            &pool,
            owner,
            saved.id,
            first.id,
            0,
            GroceryItemChange::Checked(true)
        ),
        change_item(
            &pool,
            owner,
            saved.id,
            second.id,
            0,
            GroceryItemChange::Checked(true)
        )
    );
    a.unwrap();
    b.unwrap();
    change_item(
        &pool,
        owner,
        saved.id,
        first.id,
        0,
        GroceryItemChange::Checked(true),
    )
    .await
    .unwrap();
    assert!(matches!(
        change_item(
            &pool,
            owner,
            saved.id,
            first.id,
            0,
            GroceryItemChange::Checked(false)
        )
        .await,
        Err(GroceryError::Conflict)
    ));
    assert!(matches!(
        change_item(
            &pool,
            owner,
            saved.id,
            first.id,
            0,
            GroceryItemChange::Fields(fields("Old stale text", "1", "bottle"))
        )
        .await,
        Err(GroceryError::Conflict)
    ));
    assert!(matches!(
        delete(&pool, owner, saved.id, 0).await,
        Err(GroceryError::Conflict)
    ));
    assert_eq!(
        get(&pool, owner, saved.id)
            .await
            .unwrap()
            .items
            .iter()
            .filter(|i| i.checked)
            .count(),
        2
    );
}

#[test]
fn test_fixture_schema_matches_declarative_grocery_definitions() {
    assert!(include_str!("../../migrations/schema.sql")
        .ends_with(include_str!("../../migrations/grocery-lists.sql")));
}
