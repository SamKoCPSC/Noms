use super::*;
use axum::{
    body::{to_bytes, Body},
    http::Request,
    routing::get,
    Router,
};
use std::io::Read;
use tower::ServiceExt;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

fn router(pool: sqlx::PgPool, storage: StorageClient, actor: Option<Uuid>) -> Router {
    let router = Router::new()
        .route("/download/{id}", get(download))
        .layer(Extension(pool))
        .layer(Extension(storage));
    if let Some(user_id) = actor {
        router.layer(Extension(AuthUser { user_id }))
    } else {
        router
    }
}
fn request(id: Uuid, query: &str) -> Request<Body> {
    Request::builder()
        .uri(format!("/download/{id}?{query}"))
        .body(Body::empty())
        .unwrap()
}
fn zip_text(zip: &mut zip::ZipArchive<Cursor<Vec<u8>>>, name: &str) -> String {
    let mut text = String::new();
    zip.by_name(name)
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    text
}

#[tokio::test]
async fn collection_html_layouts_preserve_nested_order_shared_memberships_and_offline_navigation() {
    let (_db, pool) = crate::test_utils::setup_test_db().await;
    let (owner, root, _) = super::super::collection_portability::tests::tree(&pool).await;
    let storage = StorageClient::for_test();
    let app = router(pool.clone(), storage.clone(), Some(owner.id));
    let response = app
        .clone()
        .oneshot(request(root, "layout=unified&images=false"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "text/html; charset=utf-8"
    );
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    assert!(response.headers()[header::CONTENT_DISPOSITION]
        .to_str()
        .unwrap()
        .ends_with(".html\""));
    let html = String::from_utf8(
        to_bytes(response.into_body(), MAX_BYTES)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert_eq!(html.matches("<!doctype html>").count(), 1);
    assert_eq!(html.matches("class=\"cookbook-recipe\"").count(), 2);
    assert_eq!(html.matches("href=\"#recipe-").count(), 5);
    assert!(html.contains("Empty album") && html.contains("No recipes directly"));
    assert!(
        html.find("Shared bread</h2>").unwrap()
            < html.find("Bean soup &amp; bread 🥣</h2>").unwrap()
    );
    assert!(
        html.contains("Stir gently")
            && html.contains("Weeknight")
            && html.contains("with two lines")
    );
    assert!(!html.contains("<img") && html.contains("form-action 'none'"));
    let response = app
        .clone()
        .oneshot(request(root, "layout=individual&images=false"))
        .await
        .unwrap();
    assert_eq!(response.headers()[header::CONTENT_TYPE], "application/zip");
    let mut zip = zip::ZipArchive::new(Cursor::new(
        to_bytes(response.into_body(), MAX_BYTES)
            .await
            .unwrap()
            .to_vec(),
    ))
    .unwrap();
    assert_eq!(zip.len(), 3);
    let index = zip_text(&mut zip, "index.html");
    assert_eq!(index.matches("href=\"recipe-").count(), 5);
    for name in ["recipe-001.html", "recipe-002.html"] {
        let html = zip_text(&mut zip, name);
        assert!(html.starts_with("<!doctype html>") && html.ends_with("</html>"));
        assert_eq!(html.matches("<article>").count(), 1);
        assert!(!html.contains("<img") && !html.contains("<script"));
    }
    for query in ["layout=invalid", "layout=unified&images=maybe", "extra=yes"] {
        assert_eq!(
            app.clone()
                .oneshot(request(root, query))
                .await
                .unwrap()
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    // A truly empty tree still downloads a useful index in either layout.
    sqlx::query("DELETE FROM collection_recipes WHERE collection_id IN (SELECT id FROM collections WHERE owner_id=$1)").bind(owner.id).execute(&pool).await.unwrap();
    for layout in [Layout::Unified, Layout::Individual] {
        assert!(!prepare(&pool, &storage, owner.id, root, layout, false)
            .await
            .unwrap()
            .is_empty());
    }
}

#[tokio::test]
async fn collection_html_authorizes_before_any_image_reads_and_rejects_revoked_memberships() {
    let (_db, pool) = crate::test_utils::setup_test_db().await;
    let (owner, root, recipes) = super::super::collection_portability::tests::tree(&pool).await;
    let mock = MockServer::start().await;
    let storage = StorageClient::for_test_with_endpoint_and_public_url(
        &mock.uri(),
        "https://saved.example/media",
    );
    for (actor, expected) in [
        (None, StatusCode::UNAUTHORIZED),
        (Some(Uuid::new_v4()), StatusCode::BAD_REQUEST),
    ] {
        let response = router(pool.clone(), storage.clone(), actor)
            .oneshot(request(root, "layout=individual&images=true"))
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "private, no-store"
        );
        assert!(!response.headers().contains_key(header::CONTENT_DISPOSITION));
    }
    sqlx::query("UPDATE recipes SET visibility='private' WHERE id=$1")
        .bind(recipes[1])
        .execute(&pool)
        .await
        .unwrap();
    let response = router(pool.clone(), storage.clone(), Some(owner.id))
        .oneshot(request(root, "layout=unified&images=true"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = String::from_utf8(
        to_bytes(response.into_body(), MAX_BYTES)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(!body.contains("Shared bread"));
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn collection_html_embeds_saved_images_warns_about_missing_and_never_fetches_external_urls() {
    let (_db, pool) = crate::test_utils::setup_test_db().await;
    let mock = MockServer::start().await;
    let storage = StorageClient::for_test_with_endpoint_and_public_url(
        &mock.uri(),
        "https://saved.example/media",
    );
    let (recipe, owner) = super::super::recipe_archive::tests::source(&pool, &storage).await;
    let root = crate::db::insert_collection(
        &pool,
        owner,
        None,
        "Images",
        None,
        crate::types::CollectionVisibility::Private,
        None,
    )
    .await
    .unwrap();
    crate::db::add_recipe_to_collection(&pool, root.id, owner, recipe.id)
        .await
        .unwrap();
    let mut data = crate::db::read_collection_export(&pool, owner, root.id)
        .await
        .unwrap();
    for layout in [Layout::Unified, Layout::Individual] {
        assert!(build(&storage, &data, layout, false, MAX_BYTES)
            .await
            .is_ok());
        assert!(build(&storage, &data, layout, true, 512).await.is_err());
    }
    assert!(mock.received_requests().await.unwrap().is_empty());
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200).set_body_bytes(b"RIFF0000WEBPsynthetic".as_slice()),
        )
        .mount(&mock)
        .await;
    for layout in [Layout::Unified, Layout::Individual] {
        let bytes = build(&storage, &data, layout, true, MAX_BYTES)
            .await
            .unwrap();
        let html = if layout == Layout::Unified {
            String::from_utf8(bytes).unwrap()
        } else {
            zip_text(
                &mut zip::ZipArchive::new(Cursor::new(bytes)).unwrap(),
                "recipe-001.html",
            )
        };
        assert_eq!(html.matches("src=\"data:image/webp;base64,").count(), 3);
        assert!(html.contains("externally linked") && !html.contains("external.example"));
        assert!(html.contains("Some images are not in this copy"));
    }
    assert_eq!(mock.received_requests().await.unwrap().len(), 4);
    mock.reset().await;
    let html = String::from_utf8(
        build(&storage, &data, Layout::Unified, true, MAX_BYTES)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(!html.contains("<img") && html.contains("could not be accessed"));
    data.collections[0].name = "</title><script>alert(1)</script>".into();
    data.collections[0].description = Some("<img src=x onerror=alert(1)>".into());
    data.recipes[0].0.title = "../../<svg onload=alert(1)>".into();
    let bytes = build(&storage, &data, Layout::Individual, false, MAX_BYTES)
        .await
        .unwrap();
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    for name in ["index.html", "recipe-001.html"] {
        let html = zip_text(&mut zip, name);
        assert!(
            !html.contains("<script>")
                && !html.contains("<img src=x")
                && !html.contains("<svg onload")
        );
        assert!(html.contains("&lt;"));
    }
}
