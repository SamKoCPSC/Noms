use super::{pantry, pantry_recipes::find};
use crate::pantry::*;
use sqlx::PgPool;
use uuid::Uuid;

async fn user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO users(id,username,display_name,email) VALUES($1,$2,$2,$3)")
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
        .bind(id).bind(owner).bind(format!("{title} {id}")).bind(visibility).bind(serde_json::json!(ingredients))
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
        if i % 2 == 0 {
            sqlx::query("UPDATE recipes SET ingredients=ingredients || '[{\"name\":\"Parsley\",\"optional\":true}]'::jsonb WHERE id=$1")
                .bind(id).execute(&pool).await.unwrap();
        }
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
        assert!(page
            .optional_ingredients_missing
            .keys()
            .all(|id| page.recipes.iter().any(|r| r.id == *id)));
        assert!(page
            .optional_ingredients_missing
            .values()
            .all(|count| *count == 1));
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
async fn pantry_recipes_allow_optional_missing_and_keep_required_and_malformed_exclusions() {
    let (_db, pool) = crate::test_utils::setup_test_db().await;
    let owner = user(&pool).await;
    let p = kitchen(&pool, owner).await;
    let cases = [
        (
            "Full optional",
            serde_json::json!([{"name":"rice","optional":true}]),
            Some(0),
        ),
        (
            "Optional garnish",
            serde_json::json!([{"name":"Rice"},{"name":"Parsley","optional":true},{"name":"PARSLEY","optional":true}]),
            Some(1),
        ),
        (
            "Optional only",
            serde_json::json!([{"name":"Parsley","optional":true}]),
            Some(1),
        ),
        (
            "Required duplicate",
            serde_json::json!([{"name":"Milk","optional":true},{"name":"MILK"}]),
            None,
        ),
        (
            "Required duplicate first",
            serde_json::json!([{"name":"MILK"},{"name":"Milk","optional":true}]),
            None,
        ),
        (
            "Malformed flag",
            serde_json::json!([{"name":"Rice","optional":"true"}]),
            None,
        ),
        (
            "Malformed name",
            serde_json::json!([{"optional":true}]),
            None,
        ),
        (
            "Legacy required",
            serde_json::json!([{"name":"Milk"}]),
            None,
        ),
    ];
    let mut expected = Vec::new();
    for (title, ingredients, count) in cases {
        let id = recipe(&pool, owner, title, "private", &["Rice"]).await;
        sqlx::query("UPDATE recipes SET ingredients=$1 WHERE id=$2")
            .bind(ingredients)
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
        if let Some(count) = count {
            expected.push((id, count));
        }
    }
    let page = find(&pool, owner, p.id, p.revision, None).await.unwrap();
    assert_eq!(page.recipes.len(), expected.len());
    for (id, count) in expected {
        assert!(page.recipes.iter().any(|r| r.id == id));
        assert_eq!(
            page.optional_ingredients_missing
                .get(&id)
                .copied()
                .unwrap_or(0),
            count
        );
    }
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


async fn almost(pool: &PgPool, owner: Uuid, p: &PantryKitchen, cursor: Option<&str>) -> Result<PantryRecipePage, pantry::PantryError> {
    super::pantry_recipes::find_in_mode(pool, owner, p.id, p.revision, cursor, PantryRecipeMode::AlmostCookable).await
}

#[tokio::test]
async fn almost_cookable_counts_distinct_required_names_without_optional_penalties() {
    let (_db, pool) = crate::test_utils::setup_test_db().await;
    let owner = user(&pool).await;
    let p = kitchen(&pool, owner).await;
    let cases = [
        ("Complete", serde_json::json!([{"name":"Rice"}]), None),
        ("Only optional missing", serde_json::json!([{"name":"Parsley","optional":true}]), None),
        ("One missing", serde_json::json!([{"name":"rice"},{"name":"Milk"},{"name":"Basil","optional":true}]), Some(vec!["Milk"])),
        ("Two missing", serde_json::json!([{"name":"Milk"},{"name":"MILK"},{"name":"Jalapeño"}]), Some(vec!["Milk","Jalapeño"])),
        ("Required wins", serde_json::json!([{"name":"Milk","optional":true},{"name":"MILK"},{"name":"Pepper","optional":true}]), Some(vec!["Milk"])),
        ("Three missing", serde_json::json!([{"name":"Milk"},{"name":"Basil"},{"name":"Pepper"}]), None),
        ("Malformed optional", serde_json::json!([{"name":"Milk","optional":"true"}]), None),
        ("Malformed name", serde_json::json!([{"name":3}]), None),
        ("Blank name", serde_json::json!([{"name":" "}]), None),
        ("Empty", serde_json::json!([]), None),
    ];
    let mut expected = std::collections::BTreeMap::new();
    for (title, ingredients, missing) in cases {
        let id = recipe(&pool, owner, title, "private", &["Rice"]).await;
        sqlx::query("UPDATE recipes SET ingredients=$1 WHERE id=$2")
            .bind(ingredients).bind(id).execute(&pool).await.unwrap();
        if let Some(missing) = missing {
            expected.insert(id, missing.into_iter().map(str::to_string).collect::<Vec<_>>());
        }
    }
    let page = almost(&pool, owner, &p, None).await.unwrap();
    assert_eq!(page.mode, PantryRecipeMode::AlmostCookable);
    assert_eq!(page.required_ingredients_missing, expected);
    assert_eq!(page.recipes.len(), 3);
    assert_eq!(page.optional_ingredients_missing.len(), 2);
    assert!(page.next_cursor.is_none());
    let json = serde_json::to_value(&page).unwrap();
    assert!(json["recipes"][0].get("ingredients").is_none());
    assert!(json["recipes"][0].get("instructions").is_none());
}

#[tokio::test]
async fn almost_cookable_ranks_globally_and_paginates_across_missing_count_groups() {
    let (_db, pool) = crate::test_utils::setup_test_db().await;
    let owner = user(&pool).await;
    let p = kitchen(&pool, owner).await;
    let c = collection(&pool, owner, None).await;
    let mut one = Vec::new();
    let mut two = Vec::new();
    for i in 0..31 {
        let names: &[&str] = if i % 2 == 0 { &["Rice","Milk"] } else { &["Rice","Milk","Basil"] };
        let id = recipe(&pool, owner, &format!("Ranked {i}"), "private", names).await;
        save(&pool, c, id).await;
        let time = if i % 2 == 0 { "2020-01-01T00:00:00Z" } else { "2030-01-01T00:00:00Z" };
        sqlx::query("UPDATE recipes SET updated_at=$1::text::timestamptz WHERE id=$2")
            .bind(time).bind(id).execute(&pool).await.unwrap();
        if i % 2 == 0 { one.push(id); } else { two.push(id); }
    }
    one.sort_by(|a,b| b.cmp(a));
    two.sort_by(|a,b| b.cmp(a));
    let expected: Vec<_> = one.into_iter().chain(two).collect();
    let mut seen = Vec::new();
    let mut cursor = None;
    let mut sizes = Vec::new();
    loop {
        let page = almost(&pool, owner, &p, cursor.as_deref()).await.unwrap();
        sizes.push(page.recipes.len());
        assert_eq!(page.required_ingredients_missing.len(), page.recipes.len());
        assert!(page.recipes.iter().all(|r| page.required_ingredients_missing.contains_key(&r.id)));
        seen.extend(page.recipes.iter().map(|r| r.id));
        if let Some(value) = page.next_cursor.as_deref() {
            assert!(matches!(find(&pool, owner, p.id, p.revision, Some(value)).await, Err(pantry::PantryError::Invalid(_))));
        }
        cursor = page.next_cursor;
        if cursor.is_none() { break; }
        assert!(sizes.len() < 4);
    }
    assert_eq!(seen, expected);
    assert_eq!(sizes, vec![12,12,7]);
}

#[tokio::test]
async fn almost_cookable_preserves_library_access_and_refreshes_after_inventory_changes() {
    let (_db, pool) = crate::test_utils::setup_test_db().await;
    let owner = user(&pool).await;
    let other = user(&pool).await;
    let p = kitchen(&pool, owner).await;
    let root = collection(&pool, owner, None).await;
    let child = collection(&pool, owner, Some(root)).await;
    let visible = recipe(&pool, other, "Saved public", "public", &["Rice","Milk"]).await;
    let unlisted = recipe(&pool, other, "Saved unlisted", "unlisted", &["Rice","Milk"]).await;
    let hidden = recipe(&pool, other, "Saved private", "private", &["Rice","Milk"]).await;
    recipe(&pool, other, "Unsaved public", "public", &["Rice","Milk"]).await;
    for id in [visible, unlisted, hidden] { save(&pool, child, id).await; }
    save(&pool, root, visible).await;
    assert!(matches!(almost(&pool, other, &p, None).await, Err(pantry::PantryError::Missing)));
    let page = almost(&pool, owner, &p, None).await.unwrap();
    assert_eq!(page.recipes.len(), 2);
    assert!(page.recipes.iter().all(|r| [visible, unlisted].contains(&r.id)));
    assert!(!page.required_ingredients_missing.contains_key(&hidden));
    sqlx::query("UPDATE recipes SET visibility='private' WHERE id=$1")
        .bind(visible).execute(&pool).await.unwrap();
    assert_eq!(almost(&pool, owner, &p, None).await.unwrap().recipes.len(), 1);
    pantry::add(&pool, owner, p.id, PantryKind::Ingredient, Uuid::new_v4(), stock("MILK")).await.unwrap();
    assert!(matches!(almost(&pool, owner, &p, None).await, Err(pantry::PantryError::Conflict)));
    let newer = pantry::get(&pool, owner, p.id).await.unwrap();
    assert!(almost(&pool, owner, &newer, None).await.unwrap().recipes.is_empty());
    assert_eq!(find(&pool, owner, p.id, newer.revision, None).await.unwrap().recipes[0].id, unlisted);
}
