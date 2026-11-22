use super::*;
use axum::{body::to_bytes, http::Request, routing::get, Router};
use noms::vocabulary::{
    index::{Catalog, Manifest, SearchIndex, MANIFEST_PATH},
    store, SeedFile, SourceManifest,
};
use tower::ServiceExt;

fn router(pool: sqlx::PgPool, actor: Option<uuid::Uuid>) -> Router {
    let router = Router::new()
        .route(MANIFEST_PATH, get(manifest))
        .route("/api/vocabulary/ingredients/{version}", get(catalog))
        .layer(Extension(IndexService::new(pool)));
    if let Some(user_id) = actor {
        router.layer(Extension(AuthUser { user_id }))
    } else {
        router
    }
}
fn request(path: &str) -> axum::http::request::Builder {
    Request::builder().uri(path)
}

#[tokio::test]
async fn anonymous_downloads_fail_before_accessing_database_even_with_conditional_headers() {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgresql://unused:unused@127.0.0.1:1/unused")
        .unwrap();
    let app = router(pool, None);
    for url in [
        MANIFEST_PATH.to_string(),
        format!("/api/vocabulary/ingredients/{}.json", "0".repeat(64)),
    ] {
        let r = app
            .clone()
            .oneshot(
                request(&url)
                    .header(header::IF_NONE_MATCH, "*")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(r.headers()[header::CACHE_CONTROL], "no-store");
    }
}

#[tokio::test]
async fn downloads_are_compressed_versioned_conditional_and_refresh_after_seed_changes() {
    let (_db, pool) = crate::test_utils::setup_test_db().await;
    let mut seed: SeedFile = serde_json::from_str(include_str!(
        "../../data/vocabulary/adapters/foodon-cooking-base-ingredients.json"
    ))
    .unwrap();
    let sources: SourceManifest = serde_json::from_str(include_str!(
        "../../data/vocabulary/adapters/foodon-cooking-base-sources.json"
    ))
    .unwrap();
    store::seed(&pool, &seed, &sources, false).await.unwrap();
    let app = router(pool.clone(), Some(uuid::Uuid::new_v4()));
    let r = app
        .clone()
        .oneshot(request(MANIFEST_PATH).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-cache");
    let etag = r.headers()[header::ETAG].clone();
    let initial: Manifest =
        serde_json::from_slice(&to_bytes(r.into_body(), 1024 * 1024).await.unwrap()).unwrap();
    initial.validate().unwrap();
    let r = app
        .clone()
        .oneshot(
            request(MANIFEST_PATH)
                .header(header::IF_NONE_MATCH, etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NOT_MODIFIED);
    assert!(to_bytes(r.into_body(), 1024).await.unwrap().is_empty());
    let r = app
        .clone()
        .oneshot(
            request(&initial.url)
                .header(header::ACCEPT_ENCODING, "br, gzip;q=1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(r.headers()[header::CONTENT_ENCODING], "gzip");
    assert_eq!(
        r.headers()[header::CACHE_CONTROL],
        "private, max-age=31536000, immutable"
    );
    let tag = r.headers()[header::ETAG].clone();
    let bytes = to_bytes(r.into_body(), 4 * 1024 * 1024).await.unwrap();
    let mut decoded = Vec::new();
    std::io::Read::read_to_end(&mut flate2::read::GzDecoder::new(&bytes[..]), &mut decoded)
        .unwrap();
    let payload: Catalog = serde_json::from_slice(&decoded).unwrap();
    assert_eq!(payload.entries.len(), 100);
    let index = SearchIndex::new(&initial.version, payload, &initial.popularity).unwrap();
    assert_eq!(
        index.search("garbanzo")[0].id,
        store::autocomplete(&pool, "garbanzo").await.unwrap()[0].id
    );
    for word in ["description", "provenance", "source_mappings"] {
        assert!(!String::from_utf8_lossy(&decoded).contains(word));
    }
    let r = app
        .clone()
        .oneshot(
            request(&initial.url)
                .header(header::IF_NONE_MATCH, tag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NOT_MODIFIED);
    let r = app
        .clone()
        .oneshot(
            request(&initial.url)
                .header(header::ACCEPT_ENCODING, "gzip;q=0, identity")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(!r.headers().contains_key(header::CONTENT_ENCODING));
    assert_eq!(
        &to_bytes(r.into_body(), 4 * 1024 * 1024).await.unwrap()[..],
        &decoded
    );
    for (url, status) in [
        (
            "/api/vocabulary/ingredients/not-a-version.json".to_string(),
            StatusCode::BAD_REQUEST,
        ),
        (
            format!("/api/vocabulary/ingredients/{}.json", "0".repeat(64)),
            StatusCode::GONE,
        ),
    ] {
        let r = app
            .clone()
            .oneshot(request(&url).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(r.status(), status);
        assert_eq!(r.headers()[header::CACHE_CONTROL], "no-store");
    }
    seed.entries[0]
        .aliases
        .push("new browser cache identity".into());
    store::seed(&pool, &seed, &sources, false).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let r = app
        .clone()
        .oneshot(request(MANIFEST_PATH).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let changed: Manifest =
        serde_json::from_slice(&to_bytes(r.into_body(), 1024 * 1024).await.unwrap()).unwrap();
    assert_ne!(changed.version, initial.version);
    let r = app
        .oneshot(request(&changed.url).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let body = to_bytes(r.into_body(), 4 * 1024 * 1024).await.unwrap();
    let index = SearchIndex::new(
        &changed.version,
        serde_json::from_slice(&body).unwrap(),
        &changed.popularity,
    )
    .unwrap();
    assert_eq!(
        index.search("new browser cache identity")[0].id,
        seed.entries[0].id
    );
}
