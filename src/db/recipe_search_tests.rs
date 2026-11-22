use super::*;
use crate::types::TagFacetBucket;
use sqlx::Execute;

#[tokio::test]
#[ignore = "requires SEARCH_TEST_DATABASE_URL backed by the pinned ParadeDB image"]
async fn search_integration_summary_shares_index_scan_and_preserves_facet_boundaries() {
    let pool = crate::test_utils::setup_search_test_db().await;
    let owner = Uuid::new_v4();
    let other = Uuid::new_v4();
    let term = format!("facetboundary{}", crate::test_utils::uid());
    for id in [owner, other] {
        sqlx::query("INSERT INTO users(id,username,display_name,email) VALUES($1,$2,$2,$3)")
            .bind(id)
            .bind(format!("facet_{}", &id.simple().to_string()[..20]))
            .bind(format!("{id}@example.test"))
            .execute(&pool)
            .await
            .unwrap();
    }
    // Exact lower/upper boundaries, one unknown component, both unknown, and
    // public/own/other-private recipes exercise the aggregate's SQL null rules.
    let fixtures = [
        (owner, "private", Some(0), None, Some(1)),
        (owner, "unlisted", Some(29), Some(0), Some(2)),
        (owner, "public", Some(10), Some(20), Some(3)),
        (owner, "public", Some(59), None, Some(4)),
        (owner, "public", None, Some(60), Some(5)),
        (owner, "public", Some(119), Some(0), Some(8)),
        (owner, "public", None, Some(120), Some(9)),
        (owner, "public", Some(90), Some(30), Some(10)),
        (owner, "public", None, None, None),
        (other, "public", Some(30), None, Some(3)),
        (other, "private", Some(30), None, Some(3)),
    ];
    for (index, (user, visibility, prep, cook, servings)) in fixtures.into_iter().enumerate() {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO recipes(id,user_id,title,visibility,prep_time_minutes,cook_time_minutes,servings) VALUES($1,$2,$3,$4,$5,$6,$7)")
            .bind(id).bind(user).bind(format!("{term} recipe {index}")).bind(visibility).bind(prep).bind(cook).bind(servings)
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO recipe_tags(recipe_id,tag) VALUES($1,$2)")
            .bind(id)
            .bind(&term)
            .execute(&pool)
            .await
            .unwrap();
    }
    let request = RecipeSearchRequest {
        text: Some(term.clone()),
        tags_all: vec![term.clone()],
        ..Default::default()
    };
    let response = search_recipes(&pool, owner, &request).await.unwrap();
    assert_eq!(response.total_count, 10);
    assert_eq!(
        response
            .facets
            .visibility
            .iter()
            .map(|b| b.count)
            .collect::<Vec<_>>(),
        [1, 1, 8]
    );
    assert_eq!(
        response
            .facets
            .total_time
            .iter()
            .map(|b| b.count)
            .collect::<Vec<_>>(),
        [2, 3, 2, 2]
    );
    assert_eq!(
        response
            .facets
            .servings
            .iter()
            .map(|b| b.count)
            .collect::<Vec<_>>(),
        [2, 3, 2, 2]
    );
    assert_eq!(
        response.facets.tags,
        [TagFacetBucket {
            tag: term.clone(),
            count: 10
        }]
    );

    let ranged = RecipeSearchRequest {
        total_time: Some(IntegerRange {
            lower_inclusive: Some(30),
            upper_exclusive: Some(60),
        }),
        servings: Some(IntegerRange {
            lower_inclusive: None,
            upper_exclusive: Some(5),
        }),
        ..request.clone()
    };
    let response = search_recipes(&pool, owner, &ranged).await.unwrap();
    assert_eq!(response.total_count, 3);
    assert_eq!(
        response
            .facets
            .total_time
            .iter()
            .map(|b| b.count)
            .collect::<Vec<_>>(),
        [2, 3, 0, 0]
    );
    assert_eq!(
        response
            .facets
            .servings
            .iter()
            .map(|b| b.count)
            .collect::<Vec<_>>(),
        [0, 3, 0, 0]
    );
    assert_eq!(response.facets.tags[0].count, 3);

    let mine = RecipeSearchRequest {
        visibility: RecipeSearchVisibility::MineOnly,
        ..request.clone()
    };
    let response = search_recipes(&pool, owner, &mine).await.unwrap();
    assert_eq!(response.total_count, 9);
    assert_eq!(
        response
            .facets
            .visibility
            .iter()
            .map(|b| b.count)
            .collect::<Vec<_>>(),
        [1, 1, 8]
    );
    assert_eq!(
        response
            .facets
            .total_time
            .iter()
            .map(|b| b.count)
            .collect::<Vec<_>>(),
        [2, 2, 2, 2]
    );

    let empty = RecipeSearchRequest {
        text: Some(format!("{term} missing")),
        ..request.clone()
    };
    let response = search_recipes(&pool, owner, &empty).await.unwrap();
    assert_eq!(response.total_count, 0);
    assert!(response.results.is_empty() && response.facets.tags.is_empty());
    assert!(response.facets.visibility.iter().all(|b| b.count == 0));
    assert!(response.facets.total_time.iter().all(|b| b.count == 0));
    assert!(response.facets.servings.iter().all(|b| b.count == 0));

    let mut builder = summary_query(Some(owner), &request);
    let mut query = builder.build();
    let sql = format!("EXPLAIN (ANALYZE, BUFFERS) {}", query.sql());
    let arguments = query.take_arguments().unwrap().unwrap();
    let plan = sqlx::query_scalar_with::<Postgres, String, _>(&sql, arguments)
        .fetch_all(&pool)
        .await
        .unwrap()
        .join("\n");
    assert!(
        plan.contains("CTE candidates") && plan.contains("idx_recipes_search"),
        "{plan}"
    );
    assert_eq!(
        plan.matches("Custom Scan (ParadeDB Base Scan)").count(),
        1,
        "{plan}"
    );
    eprintln!("Shared facet plan:\n{plan}");
    sqlx::query("DELETE FROM users WHERE id = ANY($1)")
        .bind(vec![owner, other])
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}
