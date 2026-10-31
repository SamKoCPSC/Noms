use super::*;
use sqlx::Execute;

#[tokio::test]
#[ignore = "requires SEARCH_TEST_DATABASE_URL backed by the pinned ParadeDB image"]
async fn search_integration_recipe_title_suggestions_are_live_distinct_bounded_and_authorized() {
    let pool = crate::test_utils::setup_search_test_db().await;
    let viewer = Uuid::new_v4();
    let other = Uuid::new_v4();
    let term = format!("titlecheck{}", crate::test_utils::uid());
    for user in [viewer, other] {
        sqlx::query(
            "INSERT INTO users(id,username,display_name,email) VALUES($1,$2,'Title test',$3)",
        )
        .bind(user)
        .bind(format!("title_{}", &user.simple().to_string()[..20]))
        .bind(format!("{user}@example.test"))
        .execute(&pool)
        .await
        .unwrap();
    }
    async fn add(pool: &PgPool, owner: Uuid, title: &str, visibility: &str, body: &str) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO recipes(id,user_id,title,visibility,description,ingredients,instructions) VALUES($1,$2,$3,$4,$5,jsonb_build_array(jsonb_build_object('name',$5::text)),jsonb_build_array(jsonb_build_object('text',$5::text)))")
            .bind(id).bind(owner).bind(title).bind(visibility).bind(body).execute(pool).await.unwrap();
        id
    }
    add(&pool, other, &term, "public", "").await;
    for _ in 0..20 {
        add(&pool, other, &format!("{term} Curry"), "public", "").await;
    }
    add(
        &pool,
        other,
        &format!("  {}   CURRY  ", term.to_uppercase()),
        "public",
        "",
    )
    .await;
    for i in 0..10 {
        add(&pool, other, &format!("{term} Dish {i}"), "public", "").await;
    }
    add(&pool, other, &format!("Other {term} curry"), "public", "").await;
    add(
        &pool,
        viewer,
        &format!("{term} Owner private"),
        "private",
        "",
    )
    .await;
    add(
        &pool,
        viewer,
        &format!("{term} Owner unlisted"),
        "unlisted",
        "",
    )
    .await;
    add(
        &pool,
        other,
        &format!("{term} Hidden private"),
        "private",
        "",
    )
    .await;
    add(
        &pool,
        other,
        &format!("{term} Hidden unlisted"),
        "unlisted",
        "",
    )
    .await;
    add(
        &pool,
        other,
        "Generic dessert",
        "public",
        &format!("{term} bodyonly"),
    )
    .await;

    let rows = suggest_recipe_titles(&pool, None, &term).await.unwrap();
    assert_eq!(rows.len(), crate::search::MAX_AUTOCOMPLETE_SUGGESTIONS);
    assert_eq!(rows[0], term);
    let keys: std::collections::HashSet<_> = rows.iter().map(|s| s.to_lowercase()).collect();
    assert_eq!(keys.len(), rows.len());
    assert!(rows.iter().all(|s| s.to_lowercase().starts_with(&term)));
    let curry = suggest_recipe_titles(&pool, None, &format!("{term} curry"))
        .await
        .unwrap();
    assert_eq!(curry.len(), 2);
    assert_eq!(curry[0].to_lowercase(), format!("{term} curry"));
    assert!(curry
        .iter()
        .all(|s| !s.starts_with(' ') && !s.contains("  ")));
    assert!(
        suggest_recipe_titles(&pool, None, &format!("{term} bodyonly"))
            .await
            .unwrap()
            .is_empty()
    );
    for visibility in ["private", "unlisted"] {
        let query = format!("{term} Owner {visibility}");
        assert_eq!(
            suggest_recipe_titles(&pool, Some(viewer), &query)
                .await
                .unwrap(),
            [query.clone()]
        );
        assert!(suggest_recipe_titles(&pool, None, &query)
            .await
            .unwrap()
            .is_empty());
        assert!(suggest_recipe_titles(&pool, Some(other), &query)
            .await
            .unwrap()
            .is_empty());
    }
    assert!(
        suggest_recipe_titles(&pool, Some(viewer), &format!("{term} Hidden"))
            .await
            .unwrap()
            .is_empty()
    );
    assert!(!suggest_recipe_titles(&pool, None, &format!("{term} urr"))
        .await
        .unwrap()
        .is_empty());
    assert!(
        suggest_recipe_titles(&pool, None, &format!("{term}%' OR 1=1 --"))
            .await
            .unwrap()
            .is_empty()
    );
    for invalid in [
        "a".to_string(),
        "x".repeat(crate::search::MAX_SEARCH_TEXT_CHARS + 1),
    ] {
        assert!(matches!(
            suggest_recipe_titles(&pool, Some(viewer), &invalid).await,
            Err(DbError::InvalidSearchRequest(_))
        ));
    }

    let old_title = format!("{term} Fresh update");
    let id = add(&pool, viewer, &old_title, "private", "").await;
    assert!(suggest_recipe_titles(&pool, None, &old_title)
        .await
        .unwrap()
        .is_empty());
    sqlx::query("UPDATE recipes SET visibility='public' WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        suggest_recipe_titles(&pool, None, &old_title)
            .await
            .unwrap(),
        [old_title.clone()]
    );
    let new_title = format!("{term} Renamed live");
    sqlx::query("UPDATE recipes SET title=$2 WHERE id=$1")
        .bind(id)
        .bind(&new_title)
        .execute(&pool)
        .await
        .unwrap();
    assert!(suggest_recipe_titles(&pool, None, &old_title)
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        suggest_recipe_titles(&pool, None, &new_title)
            .await
            .unwrap(),
        [new_title.clone()]
    );
    sqlx::query("DELETE FROM recipes WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(suggest_recipe_titles(&pool, None, &new_title)
        .await
        .unwrap()
        .is_empty());

    let mut builder = title_suggestion_query(Some(viewer), &term);
    let mut query = builder.build();
    let sql = format!("EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT) {}", query.sql());
    let arguments = query.take_arguments().unwrap().unwrap();
    let plan = sqlx::query_scalar_with::<Postgres, String, _>(&sql, arguments)
        .fetch_all(&pool)
        .await
        .unwrap()
        .join("\n");
    assert!(
        plan.contains("Custom Scan") && plan.contains("idx_recipes_search"),
        "{plan}"
    );
    eprintln!("Title suggestion plan:\n{plan}");
    sqlx::query("DELETE FROM users WHERE id = ANY($1)")
        .bind(vec![viewer, other])
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}
