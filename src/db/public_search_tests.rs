use super::*;
use crate::types::*;
use sqlx::PgPool;

struct Fixtures {
    owner: Uuid,
    other: Uuid,
    needle: String,
    public_recipes: Vec<Uuid>,
    public_collections: Vec<Uuid>,
    child: Uuid,
    grandchild: Uuid,
    leaf: Uuid,
    hidden_root: String,
    hidden_middle: String,
    child_name: String,
    grandchild_name: String,
    leaf_name: String,
}

async fn fixtures(pool: &PgPool) -> Fixtures {
    let needle = format!("publicscope{}", crate::test_utils::uid());
    let owner = Uuid::new_v4();
    let other = Uuid::new_v4();
    for (id, suffix) in [(owner, "owner"), (other, "other")] {
        sqlx::query("INSERT INTO users(id,username,display_name,email,bio) VALUES($1,$2,$3,$4,$5)")
            .bind(id)
            .bind(format!("{suffix}_{}", &id.simple().to_string()[..16]))
            .bind(format!("{needle} {suffix}"))
            .bind(format!("secret-{id}@example.test"))
            .bind(format!("A public profile for {needle}"))
            .execute(pool)
            .await
            .unwrap();
    }
    let mut public_recipes = Vec::new();
    let mut private_recipe = Uuid::nil();
    for (actor, suffix) in [(owner, "owner"), (other, "other")] {
        for visibility in ["public", "private", "unlisted"] {
            let id = Uuid::new_v4();
            sqlx::query("INSERT INTO recipes(id,user_id,title,visibility,servings,prep_time_minutes) VALUES($1,$2,$3,$4,$5,$6)")
                .bind(id).bind(actor).bind(format!("{needle} {suffix} {visibility} recipe"))
                .bind(visibility).bind(if visibility == "public" { 2 } else { 99 })
                .bind(if visibility == "public" { 20 } else { 900 })
                .execute(pool).await.unwrap();
            sqlx::query("INSERT INTO recipe_tags(recipe_id,tag) VALUES($1,$2)")
                .bind(id)
                .bind(if visibility == "public" {
                    "safe"
                } else {
                    "secret"
                })
                .execute(pool)
                .await
                .unwrap();
            if visibility == "public" {
                public_recipes.push(id);
            }
            if actor == owner && visibility == "private" {
                private_recipe = id;
            }
        }
    }
    let root = Uuid::new_v4();
    let child = Uuid::new_v4();
    let grandchild = Uuid::new_v4();
    let middle = Uuid::new_v4();
    let leaf = Uuid::new_v4();
    let other_public = Uuid::new_v4();
    let hidden_root = format!("{needle} Hidden root");
    let hidden_middle = format!("{needle} Hidden middle");
    let child_name = format!("{needle} Public child");
    let grandchild_name = format!("{needle} Public grandchild");
    let leaf_name = format!("{needle} Public leaf");
    for (id, actor, parent, name, visibility) in [
        (root, owner, None, hidden_root.clone(), "private"),
        (child, owner, Some(root), child_name.clone(), "public"),
        (
            grandchild,
            owner,
            Some(child),
            grandchild_name.clone(),
            "public",
        ),
        (
            middle,
            owner,
            Some(child),
            hidden_middle.clone(),
            "unlisted",
        ),
        (leaf, owner, Some(middle), leaf_name.clone(), "public"),
        (
            other_public,
            other,
            None,
            format!("{needle} Other public"),
            "public",
        ),
        (
            Uuid::new_v4(),
            other,
            None,
            format!("{needle} Other private"),
            "private",
        ),
        (
            Uuid::new_v4(),
            other,
            None,
            format!("{needle} Other unlisted"),
            "unlisted",
        ),
    ] {
        sqlx::query("INSERT INTO collections(id,owner_id,parent_collection_id,name,visibility) VALUES($1,$2,$3,$4,$5)")
            .bind(id).bind(actor).bind(parent).bind(name).bind(visibility)
            .execute(pool).await.unwrap();
    }
    sqlx::query("INSERT INTO collection_recipes(collection_id,recipe_id) VALUES($1,$2)")
        .bind(child)
        .bind(private_recipe)
        .execute(pool)
        .await
        .unwrap();
    Fixtures {
        owner,
        other,
        needle,
        public_recipes,
        public_collections: vec![child, grandchild, leaf, other_public],
        child,
        grandchild,
        leaf,
        hidden_root,
        hidden_middle,
        child_name,
        grandchild_name,
        leaf_name,
    }
}

#[tokio::test]
async fn anonymous_browse_filters_rows_counts_and_facets_before_projection() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let fixture = fixtures(&pool).await;
    let response = search_recipes(&pool, None, &RecipeSearchRequest::default())
        .await
        .unwrap();
    assert_eq!(response.total_count, 2);
    assert!(response
        .results
        .iter()
        .all(|row| fixture.public_recipes.contains(&row.recipe.id)));
    assert!(response
        .facets
        .tags
        .iter()
        .all(|bucket| bucket.tag != "secret"));
    assert!(response
        .facets
        .visibility
        .iter()
        .all(
            |bucket| bucket.visibility == RecipeVisibilityFacetValue::Public || bucket.count == 0
        ));
    let mine = search_recipes(
        &pool,
        None,
        &RecipeSearchRequest {
            visibility: RecipeSearchVisibility::MineOnly,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(mine.total_count, 0);
    assert!(mine.results.is_empty());
    let owner = search_recipes(&pool, fixture.owner, &RecipeSearchRequest::default())
        .await
        .unwrap();
    assert_eq!(owner.total_count, 4);
    let owned = search_recipes(
        &pool,
        Some(fixture.owner),
        &RecipeSearchRequest {
            visibility: RecipeSearchVisibility::MineOnly,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(owned.total_count, 3);
    let grouped = search_all(&pool, None, &AllSearchRequest::default())
        .await
        .unwrap();
    assert_eq!(grouped.recipe_total_count, 2);
    assert_eq!(grouped.collection_total_count, 4);
    assert_eq!(grouped.user_total_count, 2);
    let body = serde_json::to_string(&grouped).unwrap();
    assert!(!body.contains("secret-"));
    assert!(!body.contains(&fixture.hidden_root));
    assert!(!body.contains(&fixture.hidden_middle));
}

#[tokio::test]
async fn anonymous_collections_stop_breadcrumbs_at_inaccessible_parents_and_paginate() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    let fixture = fixtures(&pool).await;
    let response = search_collections(&pool, None, &CollectionSearchRequest::default())
        .await
        .unwrap();
    assert_eq!(response.total_count, 4);
    assert!(response
        .results
        .iter()
        .all(|row| fixture.public_collections.contains(&row.collection.id)));
    for (id, expected) in [
        (fixture.child, vec![fixture.child_name.clone()]),
        (
            fixture.grandchild,
            vec![fixture.child_name.clone(), fixture.grandchild_name.clone()],
        ),
        (fixture.leaf, vec![fixture.leaf_name.clone()]),
    ] {
        assert_eq!(
            response
                .results
                .iter()
                .find(|row| row.collection.id == id)
                .unwrap()
                .breadcrumb,
            expected
        );
    }
    let owner = search_collections(
        &pool,
        Some(fixture.owner),
        &CollectionSearchRequest::default(),
    )
    .await
    .unwrap();
    assert_eq!(owner.total_count, 6);
    assert!(owner
        .results
        .iter()
        .find(|row| row.collection.id == fixture.child)
        .unwrap()
        .breadcrumb
        .contains(&fixture.hidden_root));
    let mut seen = std::collections::HashSet::new();
    for page in 1..=4 {
        let response = search_collections(
            &pool,
            None,
            &CollectionSearchRequest {
                page,
                page_size: 1,
                sort: CollectionSearchSort::NameAscending,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(response.total_count, 4);
        assert_eq!(response.results.len(), 1);
        assert!(seen.insert(response.results[0].collection.id));
    }
    assert_eq!(seen.len(), 4);
}

#[tokio::test]
#[ignore = "requires SEARCH_TEST_DATABASE_URL backed by the pinned ParadeDB image"]
async fn search_integration_public_search_keeps_visibility_in_results_facets_and_suggestions() {
    let pool = crate::test_utils::setup_search_test_db().await;
    let fixture = fixtures(&pool).await;
    let request = RecipeSearchRequest {
        text: Some(fixture.needle.clone()),
        ..Default::default()
    };
    let recipes = search_recipes(&pool, None, &request).await.unwrap();
    assert_eq!(recipes.total_count, 2);
    assert_eq!(recipes.results.len(), 2);
    assert!(recipes.facets.tags.iter().all(|tag| tag.tag != "secret"));
    let owned = search_recipes(&pool, fixture.owner, &request)
        .await
        .unwrap();
    assert_eq!(owned.total_count, 4);
    let grouped = search_all(
        &pool,
        None,
        &AllSearchRequest {
            text: fixture.needle.clone(),
            section_limit: 10,
        },
    )
    .await
    .unwrap();
    assert_eq!(grouped.recipe_total_count, 2);
    assert_eq!(grouped.collection_total_count, 4);
    assert_eq!(grouped.user_total_count, 2);
    let collections = search_collections(
        &pool,
        None,
        &CollectionSearchRequest {
            text: Some(fixture.needle.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(collections.total_count, 4);
    let serialized = serde_json::to_string(&collections).unwrap();
    assert!(!serialized.contains(&fixture.hidden_root));
    assert!(!serialized.contains(&fixture.hidden_middle));
    for scope in [
        SearchScope::All,
        SearchScope::Recipes,
        SearchScope::Collections,
        SearchScope::Users,
    ] {
        let suggestions = autocomplete_entities(&pool, None, scope, &fixture.needle)
            .await
            .unwrap();
        let serialized = serde_json::to_string(&suggestions).unwrap();
        assert!(!serialized.contains("private recipe"));
        assert!(!serialized.contains("unlisted recipe"));
        assert!(!serialized.contains(&fixture.hidden_root));
        assert!(!serialized.contains(&fixture.hidden_middle));
        assert!(!serialized.contains("secret-"));
        for suggestion in &suggestions {
            if suggestion.kind == SearchSuggestionKind::Recipe {
                assert!(fixture.public_recipes.contains(&suggestion.id));
            } else if suggestion.kind == SearchSuggestionKind::Collection {
                assert!(fixture.public_collections.contains(&suggestion.id));
            }
        }
    }
    assert_eq!(
        suggest_recipe_titles(&pool, None, &fixture.needle)
            .await
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        suggest_recipe_titles(&pool, Some(fixture.owner), &fixture.needle)
            .await
            .unwrap()
            .len(),
        4
    );
    sqlx::query("DELETE FROM users WHERE id = ANY($1)")
        .bind(vec![fixture.owner, fixture.other])
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}
