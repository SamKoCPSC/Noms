use super::{pantry, pantry_recipes::find};
use crate::pantry::*;
use sqlx::PgPool;
use uuid::Uuid;

async fn user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO users(id,username,display_name,email) VALUES($1,$2,'Cook tester',$3)")
        .bind(id)
        .bind(format!("cook_{}", &id.simple().to_string()[..20]))
        .bind(format!("{id}@example.test"))
        .execute(pool)
        .await
        .unwrap();
    id
}
fn stock(name: &str) -> PantryStock {
    PantryStock {
        fields: PantryFields {
            name: name.into(),
            amount: "0".into(),
            unit: "kg".into(),
        },
        available: true,
    }
}
async fn kitchen(pool: &PgPool, owner: Uuid) -> PantryKitchen {
    pantry::create(
        pool,
        owner,
        PantryCreate {
            request_id: Uuid::new_v4(),
            name: "Home".into(),
            ingredients: vec![stock("Rice"), stock("Salt"), stock("ΟΣ")],
            equipment: vec![PantryStock {
                fields: PantryFields {
                    name: "Pot".into(),
                    ..Default::default()
                },
                available: true,
            }],
            ..Default::default()
        },
    )
    .await
    .unwrap()
}
async fn recipe(pool: &PgPool, owner: Uuid, title: &str, visibility: &str, names: &[&str]) -> Uuid {
    let id = Uuid::new_v4();
    let ingredients: Vec<_> = names
        .iter()
        .map(|name| serde_json::json!({"name":name,"amount":"999999","unit":"cup"}))
        .collect();
    sqlx::query("INSERT INTO recipes(id,user_id,title,visibility,ingredients,description,equipment) VALUES($1,$2,$3,$4,$5,$6,'[{\"name\":\"Oven\"}]')")
        .bind(id).bind(owner).bind(title).bind(visibility).bind(serde_json::json!(ingredients))
        .bind("description ".repeat(1000)).execute(pool).await.unwrap();
    id
}
async fn collection(pool: &PgPool, owner: Uuid, parent: Option<Uuid>) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO collections(id,owner_id,name,parent_collection_id) VALUES($1,$2,$3,$4)",
    )
    .bind(id)
    .bind(owner)
    .bind(id.to_string())
    .bind(parent)
    .execute(pool)
    .await
    .unwrap();
    id
}
async fn save(pool: &PgPool, collection: Uuid, recipe: Uuid) {
    sqlx::query("INSERT INTO collection_recipes(collection_id,recipe_id) VALUES($1,$2)")
        .bind(collection)
        .bind(recipe)
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn pantry_recipes_match_library_only_with_shared_presence_rules() {
    let (_db, pool) = crate::test_utils::setup_test_db().await;
    let owner = user(&pool).await;
    let other = user(&pool).await;
    let p = kitchen(&pool, owner).await;
    let root = collection(&pool, owner, None).await;
    let child = collection(&pool, owner, Some(root)).await;
    let stranger_collection = collection(&pool, other, None).await;
    let own = recipe(&pool, owner, "Owned", "private", &["rice", "SALT", "RICE"]).await;
    let public = recipe(&pool, other, "Saved public", "public", &["Rice"]).await;
    let unlisted = recipe(&pool, other, "Saved unlisted", "unlisted", &["salt"]).await;
    let unicode = recipe(&pool, owner, "Unicode", "private", &["ος"]).await;
    for c in [root, child] {
        save(&pool, c, own).await;
        save(&pool, c, public).await;
    }
    save(&pool, child, unlisted).await;
    for (title, visibility, names) in [
        ("Unsaved public", "public", vec!["Rice"]),
        ("Unsaved unlisted", "unlisted", vec!["Salt"]),
        ("Hidden private", "private", vec!["Rice"]),
        ("Other collection", "public", vec!["Rice"]),
    ] {
        let id = recipe(&pool, other, title, visibility, &names).await;
        if title == "Hidden private" {
            save(&pool, root, id).await;
        }
        if title == "Other collection" {
            save(&pool, stranger_collection, id).await;
        }
    }
    for names in [
        vec![],
        vec!["Milk"],
        vec!["Pot"],
        vec!["Rice "],
        vec!["Garnish (optional)"],
    ] {
        recipe(&pool, owner, "Unavailable", "private", &names).await;
    }
    let page = find(&pool, owner, p.id, p.revision, None).await.unwrap();
    let actual: std::collections::HashSet<_> = page.recipes.iter().map(|r| r.id).collect();
    assert_eq!(
        actual,
        [own, public, unlisted, unicode].into_iter().collect()
    );
    assert_eq!(page.recipes.len(), 4);
    assert!(page.next_cursor.is_none());
    assert!(page
        .recipes
        .iter()
        .all(|r| r.description.as_ref().unwrap().chars().count() == 500));
    let encoded = serde_json::to_value(page).unwrap();
    assert!(encoded["recipes"][0].get("ingredients").is_none());
    assert!(encoded["recipes"][0].get("instructions").is_none());
}

#[tokio::test]
async fn pantry_recipes_pages_are_bounded_stable_and_deduplicated() {
    let (_db, pool) = crate::test_utils::setup_test_db().await;
    let owner = user(&pool).await;
    let p = kitchen(&pool, owner).await;
    let c = collection(&pool, owner, None).await;
    let mut expected = Vec::new();
    for i in 0..30 {
        let id = recipe(&pool, owner, &format!("Ready {i}"), "private", &["Rice"]).await;
        expected.push(id);
        save(&pool, c, id).await;
    }
    for _ in 0..18 {
        recipe(&pool, owner, "Not ready", "private", &["Milk"]).await;
    }
    sqlx::query("UPDATE recipes SET updated_at='2026-01-01T00:00:00Z' WHERE user_id=$1")
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();
    expected.sort_by(|a, b| b.cmp(a));
    let mut seen = Vec::new();
    let mut cursor = None;
    let mut sizes = Vec::new();
    loop {
        let page = find(&pool, owner, p.id, p.revision, cursor.as_deref())
            .await
            .unwrap();
        sizes.push(page.recipes.len());
        seen.extend(page.recipes.iter().map(|r| r.id));
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
        assert!(sizes.len() <= 3);
    }
    assert_eq!(sizes, [12, 12, 6]);
    assert_eq!(seen, expected);
}

#[tokio::test]
async fn pantry_recipes_enforce_owner_revision_cursor_and_current_visibility() {
    let (_db, pool) = crate::test_utils::setup_test_db().await;
    let owner = user(&pool).await;
    let other = user(&pool).await;
    let p = kitchen(&pool, owner).await;
    assert!(matches!(
        find(&pool, other, p.id, p.revision, None).await,
        Err(pantry::PantryError::Missing)
    ));
    assert!(matches!(
        find(&pool, owner, Uuid::new_v4(), 0, None).await,
        Err(pantry::PantryError::Missing)
    ));
    for cursor in ["bad!".to_string(), "a".repeat(513)] {
        assert!(matches!(
            find(&pool, owner, p.id, p.revision, Some(&cursor)).await,
            Err(pantry::PantryError::Invalid(_))
        ));
    }
    let c = collection(&pool, owner, None).await;
    let id = recipe(&pool, other, "Saved", "public", &["Rice"]).await;
    save(&pool, c, id).await;
    assert_eq!(
        find(&pool, owner, p.id, p.revision, None)
            .await
            .unwrap()
            .recipes
            .len(),
        1
    );
    sqlx::query("UPDATE recipes SET visibility='private' WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(find(&pool, owner, p.id, p.revision, None)
        .await
        .unwrap()
        .recipes
        .is_empty());
    sqlx::query("UPDATE recipes SET visibility='public',ingredients='[{\"name\":\"Milk\",\"amount\":\"1\",\"unit\":\"\"}]' WHERE id=$1")
        .bind(id).execute(&pool).await.unwrap();
    assert!(find(&pool, owner, p.id, p.revision, None)
        .await
        .unwrap()
        .recipes
        .is_empty());
    sqlx::query("UPDATE recipes SET ingredients='[{\"name\":\"Rice\",\"amount\":\"1\",\"unit\":\"\"}]' WHERE id=$1")
        .bind(id).execute(&pool).await.unwrap();
    pantry::change(
        &pool,
        owner,
        p.id,
        PantryKind::Ingredient,
        p.entries[0].id,
        0,
        PantryChange::Availability(false),
    )
    .await
    .unwrap();
    assert!(matches!(
        find(&pool, owner, p.id, p.revision, None).await,
        Err(pantry::PantryError::Conflict)
    ));
    let newer = pantry::get(&pool, owner, p.id).await.unwrap();
    assert!(find(&pool, owner, p.id, newer.revision, None)
        .await
        .unwrap()
        .recipes
        .is_empty());
    pantry::add(
        &pool,
        owner,
        p.id,
        PantryKind::Ingredient,
        Uuid::new_v4(),
        stock("rICE"),
    )
    .await
    .unwrap();
    let newer = pantry::get(&pool, owner, p.id).await.unwrap();
    assert_eq!(
        find(&pool, owner, p.id, newer.revision, None)
            .await
            .unwrap()
            .recipes
            .len(),
        1
    );
    sqlx::query("DELETE FROM collection_recipes WHERE recipe_id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(find(&pool, owner, p.id, newer.revision, None)
        .await
        .unwrap()
        .recipes
        .is_empty());
}
