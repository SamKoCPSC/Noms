use super::*;
use axum::{body::Body, http::Request, routing::post, Router};
use tower::ServiceExt;
// Public synthetic Base64 fixture, not a deployment credential.
const SECRET: &str = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=";

#[test]
fn staging_login_matches_supported_secret_formats_exactly() {
    let minimum = "0123456789ABCDEFGHIJKLMNOPQRSTUV";
    let maximum = minimum.repeat(16);
    for secret in [
        SECRET,
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_abcdef-",
        "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ+/abcd==",
        minimum,
        maximum.as_str(),
    ] {
        let config = StagingLoginConfig::new(secret);
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            format!("Bearer {secret}").parse().unwrap(),
        );
        assert!(config.accepts(&headers));
        for invalid in [
            format!("Bearer {}", &secret[1..]),
            format!("Bearer z{}", &secret[1..]),
            format!("Bearer {secret} "),
            format!("Bearer {secret}\t"),
            format!("Bearer {}", "a".repeat(513)),
            format!("bearer {secret}"),
        ] {
            headers.insert(header::AUTHORIZATION, invalid.parse().unwrap());
            assert!(!config.accepts(&headers));
        }
        headers.insert(
            header::AUTHORIZATION,
            format!("Bearer {secret}").parse().unwrap(),
        );
        headers.append(
            header::AUTHORIZATION,
            format!("Bearer {secret}").parse().unwrap(),
        );
        assert!(!config.accepts(&headers));
    }
}

fn router(pool: sqlx::PgPool, enabled: bool) -> Router {
    let router = Router::new()
        .route("/auth/staging/session", post(create_test_session))
        .layer(Extension(pool));
    if enabled {
        router.layer(Extension(StagingLoginConfig::new(SECRET)))
    } else {
        router
    }
}
fn request() -> axum::http::request::Builder {
    Request::builder()
        .method("POST")
        .uri("/auth/staging/session")
        .header(header::HOST, "staging.nomsrecipes.com")
        .header(header::AUTHORIZATION, format!("Bearer {SECRET}"))
}

#[tokio::test]
async fn staging_login_disabled_and_invalid_requests_do_not_access_database() {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .unwrap();
    let response = router(pool.clone(), false)
        .oneshot(request().body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    let app = router(pool, true);
    let cases = [
        (
            Request::builder()
                .method("POST")
                .uri("/auth/staging/session"),
            StatusCode::UNAUTHORIZED,
        ),
        (
            request().header(header::AUTHORIZATION, "Bearer wrong"),
            StatusCode::UNAUTHORIZED,
        ),
        (
            Request::builder()
                .method("POST")
                .uri("/auth/staging/session")
                .header(header::ORIGIN, "https://nomsrecipes.com")
                .header(header::AUTHORIZATION, format!("Bearer {SECRET}")),
            StatusCode::FORBIDDEN,
        ),
        (
            request().header(header::ORIGIN, "https://evil.test"),
            StatusCode::FORBIDDEN,
        ),
        (
            request().header(header::ORIGIN, "null"),
            StatusCode::FORBIDDEN,
        ),
        (
            request()
                .header(header::ORIGIN, ORIGIN)
                .header(header::ORIGIN, ORIGIN),
            StatusCode::FORBIDDEN,
        ),
        (
            request().uri("/auth/staging/session?user_id=someone-else"),
            StatusCode::BAD_REQUEST,
        ),
    ];
    for (builder, status) in cases {
        let response = app
            .clone()
            .oneshot(builder.body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        assert!(!response.headers().contains_key(header::SET_COOKIE));
    }
    let response = app
        .clone()
        .oneshot(
            request()
                .body(Body::from("{\"user_id\":\"someone\"}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/auth/staging/session")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn staging_login_issues_real_scoped_sessions_and_logout_revocation_works() {
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    super::super::session::set_test_secret(
        "staging-session-signing-secret-for-isolated-tests-only",
    );
    let app = router(pool.clone(), true);
    for _ in 0..2 {
        let response = app
            .clone()
            .oneshot(
                request()
                    .header(header::ORIGIN, ORIGIN)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let cookie = cookie::Cookie::parse(
            response.headers()[header::SET_COOKIE]
                .to_str()
                .unwrap()
                .to_owned(),
        )
        .unwrap();
        assert_eq!(cookie.secure(), Some(true));
        assert_eq!(cookie.http_only(), Some(true));
        assert_eq!(cookie.same_site(), Some(cookie::SameSite::Lax));
        assert_eq!(cookie.domain(), None);
        assert_eq!(cookie.path(), Some("/"));
        assert_eq!(cookie.max_age().unwrap().whole_seconds(), 900);
        assert_eq!(
            super::super::session::verify_session(&pool, cookie.value())
                .await
                .unwrap(),
            TEST_USER_ID
        );
        // Run the normal auth middleware and profile endpoint with the issued cookie.
        let guarded = Router::new()
            .route(
                "/dashboard",
                axum::routing::get(|| async { StatusCode::OK }),
            )
            .layer(axum::middleware::from_fn_with_state(
                pool.clone(),
                crate::middleware::auth::handle_auth,
            ));
        let admitted = guarded
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/dashboard")
                    .header(
                        header::COOKIE,
                        format!("{}={}", cookie.name(), cookie.value()),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(admitted.status(), StatusCode::OK);
        std::env::set_var(
            "DIOXUS_PUBLIC_PATH",
            format!("{}/assets", env!("CARGO_MANIFEST_DIR")),
        );
        let full_app =
            crate::build_dioxus_router(pool.clone(), crate::storage::StorageClient::for_test())
                .layer(Extension(crate::auth::staging_context::StagingEnvironment(
                    true,
                )));
        let rendered = full_app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(
                        header::COOKIE,
                        format!("{}={}", cookie.name(), cookie.value()),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(rendered.status(), StatusCode::OK);
        let html = String::from_utf8(
            to_bytes(rendered.into_body(), 2 * 1024 * 1024)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        assert!(html.contains("Staging environment — test data only"));
        assert!(html.contains("Codex staging tester"));
        assert!(!html.contains(SECRET));
        super::super::session::revoke_session(&pool, cookie.value())
            .await
            .unwrap();
        let refused = guarded
            .oneshot(
                Request::builder()
                    .uri("/dashboard")
                    .header(
                        header::COOKIE,
                        format!("{}={}", cookie.name(), cookie.value()),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(refused.status(), StatusCode::FOUND);
        assert!(super::super::session::verify_session(&pool, cookie.value())
            .await
            .is_err());
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM users WHERE id=$1")
        .bind(TEST_USER_ID)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    let identities: i64 =
        sqlx::query_scalar("SELECT count(*) FROM oauth_accounts WHERE user_id=$1")
            .bind(TEST_USER_ID)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(identities, 0);
}

#[tokio::test]
async fn staging_login_does_not_take_over_matching_normal_accounts_and_rolls_back_session_failure()
{
    let (_database, pool) = crate::test_utils::setup_test_db().await;
    super::super::session::set_test_secret(
        "staging-session-signing-secret-for-isolated-tests-only",
    );
    crate::db::insert_user(
        &pool,
        "codex_staging_tester",
        "Existing account",
        "normal@test.invalid",
        None,
    )
    .await
    .unwrap();
    let response = router(pool.clone(), true)
        .oneshot(request().body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM sessions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    sqlx::query("DELETE FROM users")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("CREATE FUNCTION reject_staging_session() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'test-only rejection'; END $$").execute(&pool).await.unwrap();
    sqlx::query("CREATE TRIGGER reject_staging_session BEFORE INSERT ON sessions FOR EACH ROW EXECUTE FUNCTION reject_staging_session()").execute(&pool).await.unwrap();
    let response = router(pool.clone(), true)
        .oneshot(request().body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM users WHERE id=$1")
        .bind(TEST_USER_ID)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}
