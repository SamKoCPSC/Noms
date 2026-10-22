use dioxus::prelude::*;
#[cfg(feature = "server")]
use dioxus::server::{DioxusRouterExt, ServeConfig};
use manganis::CssAssetOptions;

use noms::vocabulary;

mod api;
mod auth;
mod collection_json;
mod components;
#[cfg(feature = "server")]
mod config;
#[cfg(feature = "server")]
mod db;
#[cfg(feature = "server")]
mod middleware;
mod pages;
mod recipe_html;
mod recipe_json;
mod routing;
mod search;
mod search_url;
#[cfg(feature = "server")]
mod security;
#[cfg(feature = "server")]
mod storage;
#[cfg(all(feature = "server", test))]
mod test_utils;
mod types;
#[cfg(all(feature = "server", test))]
mod user_quota_tests;
mod utils;
#[cfg(feature = "server")]
mod worker;

use auth::context::build_context_from_fullstack;
use components::{AppLayout, ErrorFallback};
use pages::{
    About, CollectionDetail, CollectionEdit, CollectionList, CollectionNew, CommunityHome,
    Dashboard, Explore, Home, HowTo, Login, MobileYou, NotFound, RecipeDetail, RecipeDraftEdit,
    RecipeEdit, RecipeImport, RecipeNew, RecipeVersionDetail, RecipeVersionGraph, Search,
    SettingsAccounts, SettingsProfile, UserProfile,
};

// The route inventory is shared by the generated renderer and the raw-query boundary.
#[rustfmt::skip]
routing::define_app_routes! {
#[derive(Debug, Clone, Routable, PartialEq)]
enum GeneratedRoute {
    #[layout(AppLayout)]
        #[route("/")]
        Home {},
        #[route("/home")]
        CommunityHome {},
        #[route("/about")]
        About {},
        #[route("/how-to")]
        HowTo {},
        #[route("/login")]
        Login {},
        #[route("/dashboard")]
        Dashboard {},
        #[route("/recipes/new")]
        RecipeNew {},
        #[route("/recipes/import")]
        RecipeImport {},
        #[route("/recipes/:id")]
        RecipeDetail { id: String },
        #[route("/recipes/:id/versions/:version_number")]
        RecipeVersionDetail { id: String, version_number: i32 },
        #[route("/recipes/:id/version-graph")]
        RecipeVersionGraph { id: String },
        #[route("/recipes/:id/edit")]
        RecipeEdit { id: String },
        #[route("/recipes/drafts/:id/edit")]
        RecipeDraftEdit { id: String },
        #[route("/collections")]
        CollectionList {},
        #[route("/collections/new?:parent")]
        CollectionNew { parent: Option<uuid::Uuid> },
        #[route("/collections/:id")]
        CollectionDetail { id: String },
        #[route("/collections/:id/edit")]
        CollectionEdit { id: String },
        #[route("/explore")]
        Explore {},
        #[route("/search?:..query")]
        Search { query: crate::search_url::SearchRouteQuery },
        #[route("/u/:username")]
        UserProfile { username: String },
        #[redirect("/settings", || Self::SettingsProfile {})]
        #[route("/settings/profile")]
        #[route("/settings/profile")]
        SettingsProfile {},
        #[route("/you")]
        MobileYou {},
        #[route("/settings/accounts")]
        SettingsAccounts {},
        // Catch-all: matches any route not defined above
        #[route("/:..segments")]
        NotFound { segments: Vec<String> },
}
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!(
    "/assets/main.css",
    CssAssetOptions::new().with_minify(false)
);
const TAILWIND_CSS: Asset = asset!(
    "/assets/tailwind.css",
    CssAssetOptions::new().with_minify(false)
);
const COLLECTIONS_CSS: Asset = asset!(
    "/assets/collections.css",
    CssAssetOptions::new().with_minify(false)
);
const COLLECTION_FORMS_CSS: Asset = asset!(
    "/assets/collection-forms.css",
    CssAssetOptions::new().with_minify(false)
);

const MOBILE_CSS: Asset = asset!("/assets/mobile.css");
const FREDOKA: Asset = asset!("/assets/fonts/Fredoka.ttf");
const NUNITO: Asset = asset!("/assets/fonts/Nunito.ttf");

#[cfg(feature = "server")]
fn main() {
    dioxus::logger::initialize_default();

    let runtime = tokio::runtime::Runtime::new().expect("failed to initialize the server runtime");

    runtime.block_on(async move {
        let router = build_application_router().await;
        let address = dioxus::cli_config::fullstack_address_or_localhost();
        let listener = tokio::net::TcpListener::bind(address)
            .await
            .unwrap_or_else(|error| panic!("failed to bind server to {address}: {error}"));

        tracing::info!(%address, "Noms server listening");
        serve_router_with_connect_info(listener, router, std::future::pending())
            .await
            .expect("application server failed");
    });
}

#[cfg(feature = "server")]
async fn build_application_router() -> axum::Router {
    // Validate the complete deployment contract before opening database or
    // storage connections. Local development remains explicit via NOMS_ENV.
    let runtime_config = config::RuntimeConfig::from_environment()
        .unwrap_or_else(|error| panic!("application startup rejected: {error}"));
    tracing::info!(
        deployment_mode = ?runtime_config.mode(),
        hsts_enabled = runtime_config.hsts_enabled(),
        host_only_session_cookie = runtime_config.cookie_domain().is_none(),
        "Validated runtime security configuration"
    );
    let (google_client, github_client) =
        auth::oauth::build_oauth_clients(runtime_config.base_url());

    db::init_pool().await;
    let pool = db::get_pool();

    storage::init_storage().await;
    let storage_client = storage::get_storage_client();

    {
        let pool = pool.clone();
        let storage = storage_client.clone();
        let config = worker::orphan_cleanup::OrphanCleanupConfig::from_env();
        worker::orphan_cleanup::spawn_orphan_cleanup(pool, storage, config);
    }

    let rate_limit = middleware::rate_limit::RateLimitState::from_environment()
        .unwrap_or_else(|error| panic!("invalid OAuth rate-limit configuration: {error}"));
    let state = auth::oauth::AppState {
        pool: pool.clone(),
        base_url_origin: url::Url::parse(runtime_config.base_url())
            .expect("validated BASE_URL should remain parseable")
            .origin(),
        auth_security_metrics: auth::oauth::AuthSecurityMetrics::default(),
        google_client,
        github_client,
        http_client: reqwest::Client::new(),
        google_userinfo_url: std::env::var("GOOGLE_USERINFO_URL")
            .unwrap_or_else(|_| "http://localhost:8082/google/userinfo".to_string()),
        github_userinfo_url: std::env::var("GITHUB_USERINFO_URL").unwrap_or_else(|_| {
            let api_url = std::env::var("GITHUB_API_URL")
                .unwrap_or_else(|_| "https://api.github.com".to_string());
            format!("{api_url}/user")
        }),
        github_emails_url: std::env::var("GITHUB_EMAILS_URL").unwrap_or_else(|_| {
            let api_url = std::env::var("GITHUB_API_URL")
                .unwrap_or_else(|_| "https://api.github.com".to_string());
            format!("{api_url}/user/emails")
        }),
        github_extra_scopes: auth::oauth::configured_github_extra_scopes(),
        rate_limit: rate_limit.clone(),
        storage: storage_client.clone(),
    };

    let dioxus_router =
        build_dioxus_router(pool, storage_client).layer(axum::middleware::from_fn_with_state(
            rate_limit.clone(),
            middleware::rate_limit::resolved_client_ip_middleware,
        ));

    // The two endpoint classes deliberately have independent in-memory windows.
    // Each application replica owns its own limiter and cleanup task.
    let oauth_router = build_oauth_router(state);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            rate_limit.cleanup();
        }
    });

    oauth_router
        .merge(dioxus_router)
        .layer(axum::Extension(auth::local_origin::LocalCanonicalOrigin(
            runtime_config.local_loopback_origin(),
        )))
        .layer(axum::middleware::from_fn_with_state(
            runtime_config.clone(),
            middleware::local_origin::canonicalize_local_origin,
        ))
        .layer(axum::middleware::from_fn_with_state(
            runtime_config.security_headers(),
            config::security_headers_middleware,
        ))
}

#[cfg(feature = "server")]
fn build_dioxus_router(pool: sqlx::PgPool, storage_client: storage::StorageClient) -> axum::Router {
    let application_routes = axum::Router::new()
        .route(
            "/api/user_profile",
            axum::routing::get(auth::user_profile::handle_user_profile),
        )
        .route(
            "/api/upload",
            axum::routing::post(api::upload::upload_handler),
        )
        .route(
            "/api/recipes/{id}/export-images",
            axum::routing::get(api::recipe_archive::download),
        )
        .route(
            "/api/collections/{id}/export-html",
            axum::routing::get(api::collection_html::download),
        )
        .with_state(auth::user_profile::UserProfileState { pool: pool.clone() })
        .serve_dioxus_application(ServeConfig::new(), App);

    // Axum only applies a layer to routes that already exist. Keep route and
    // Dioxus construction above these layers so authentication wraps the
    // complete application router. OAuth routes are composed separately and
    // retain their purpose, callback, redirect, and rate-limit controls.
    application_routes
        .layer(api::upload::server_function_body_limit_layer())
        .layer(axum::Extension(storage_client))
        .layer(axum::middleware::from_fn_with_state(
            pool.clone(),
            middleware::auth::handle_auth,
        ))
        .layer(axum::Extension(pool))
}

#[cfg(feature = "server")]
fn build_oauth_router(state: auth::oauth::AppState) -> axum::Router {
    axum::Router::new()
        .route(
            "/auth/{provider}/start",
            axum::routing::get(auth::oauth::start_handler),
        )
        .route(
            "/auth/{provider}/callback",
            axum::routing::get(auth::oauth::callback_route_handler),
        )
        .route(
            "/auth/logout",
            axum::routing::post(auth::logout::handle_logout),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::rate_limit::rate_limit_middleware,
        ))
        .with_state(state)
}

#[cfg(feature = "server")]
async fn serve_router_with_connect_info<F>(
    listener: tokio::net::TcpListener,
    router: axum::Router,
    shutdown: F,
) -> std::io::Result<()>
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown)
    .await
}

#[cfg(all(feature = "server", test))]
mod server_tests {
    use std::net::SocketAddr;

    use axum::body::Body;
    use axum::extract::ConnectInfo;
    use axum::http::{header, Request, StatusCode};
    use axum::routing::get;
    use tokio::sync::oneshot;
    use tower::ServiceExt;

    const RESOURCE_ID: &str = "550e8400-e29b-41d4-a716-446655440000";

    async fn production_dioxus_router(pool: sqlx::PgPool) -> axum::Router {
        std::env::set_var(
            "DIOXUS_PUBLIC_PATH",
            format!("{}/assets", env!("CARGO_MANIFEST_DIR")),
        );
        super::build_dioxus_router(pool, crate::storage::StorageClient::for_test())
    }

    #[tokio::test]
    async fn production_dioxus_router_redirects_anonymous_owner_routes() {
        let (_database, pool) = crate::test_utils::setup_test_db().await;
        let app = production_dioxus_router(pool).await;
        let owner_routes = [
            "/home".to_string(),
            "/dashboard".to_string(),
            "/recipes/new".to_string(),
            format!("/recipes/{RESOURCE_ID}/edit"),
            format!("/recipes/drafts/{RESOURCE_ID}/edit"),
            "/collections".to_string(),
            "/collections/new".to_string(),
            format!("/collections/{RESOURCE_ID}/edit"),
            "/settings/profile".to_string(),
            "/settings/accounts".to_string(),
            "/search?q=pasta%20salad&scope=recipes".to_string(),
        ];

        for path in owner_routes {
            let response = app
                .clone()
                .oneshot(
                    Request::get(&path)
                        .body(Body::empty())
                        .expect("owner-route request should build"),
                )
                .await
                .expect("production Dioxus router should respond");

            assert_eq!(response.status(), StatusCode::FOUND, "path: {path}");
            assert_eq!(
                response.headers().get(header::LOCATION),
                Some(
                    &format!(
                        "/login?redirect_uri={}",
                        percent_encoding::utf8_percent_encode(
                            &path,
                            percent_encoding::NON_ALPHANUMERIC
                        )
                    )
                    .parse()
                    .expect("redirect should be a valid header value")
                ),
                "path: {path}"
            );
        }
    }

    #[tokio::test]
    async fn production_dioxus_router_leaves_published_history_to_application_authorization() {
        let (_database, pool) = crate::test_utils::setup_test_db().await;
        let app = production_dioxus_router(pool).await;
        let path = format!("/recipes/{RESOURCE_ID}/versions/1");

        let response = app
            .oneshot(
                Request::get(&path)
                    .body(Body::empty())
                    .expect("history request should build"),
            )
            .await
            .expect("production Dioxus router should respond");

        assert_ne!(response.status(), StatusCode::FOUND);
        assert_ne!(
            response.headers().get(header::LOCATION),
            Some(&axum::http::HeaderValue::from_static("/login"))
        );
    }

    #[tokio::test]
    async fn production_dioxus_router_serializes_identity_and_refreshes_session() {
        crate::auth::session::set_test_secret("router-test-secret-32-bytes-long!!");
        let (_database, pool) = crate::test_utils::setup_test_db().await;
        let user = crate::db::insert_user(
            &pool,
            "router_auth_user",
            "Router Auth User",
            "router-auth@example.com",
            None,
        )
        .await
        .expect("test user should be created");
        let token = crate::auth::session::create_session(&pool, user.id)
            .await
            .expect("test session should be created");
        sqlx::query(
            "UPDATE sessions SET expires_at = NOW() + INTERVAL '5 minutes' WHERE user_id = $1",
        )
        .bind(user.id)
        .execute(&pool)
        .await
        .expect("test session should enter the refresh window");
        let app = production_dioxus_router(pool).await;

        let response = app
            .oneshot(
                Request::get("/settings/profile")
                    .header(
                        header::COOKIE,
                        format!("{}={token}", crate::auth::session::cookie_name()),
                    )
                    .body(Body::empty())
                    .expect("authenticated request should build"),
            )
            .await
            .expect("production Dioxus router should respond");

        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response.headers().contains_key(header::SET_COOKIE),
            "qualifying sessions should be refreshed by application middleware"
        );
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("SSR response body should be readable");
        let html = String::from_utf8_lossy(&body);
        assert!(
            html.contains("Router Auth User"),
            "authenticated identity extensions should reach Dioxus SSR"
        );
        // Dioxus only emits server hydration entries in the server-only feature
        // build used by `dx serve`; the repository's broad unit-test build also
        // enables `web`, where the transport intentionally behaves as a client.
        #[cfg(not(feature = "web"))]
        assert!(
            html.contains("noms::auth::context::AuthContext"),
            "authenticated SSR state should be serialized for client hydration"
        );
    }

    #[tokio::test]
    async fn production_listener_attaches_direct_peer_information() {
        async fn peer_address(ConnectInfo(peer): ConnectInfo<SocketAddr>) -> String {
            peer.ip().to_string()
        }

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test listener should bind");
        let address = listener
            .local_addr()
            .expect("test listener should have an address");
        let router = axum::Router::new().route("/peer", get(peer_address));
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let server = tokio::spawn(super::serve_router_with_connect_info(
            listener,
            router,
            async move {
                let _ = shutdown_rx.await;
            },
        ));

        let response = reqwest::get(format!("http://{address}/peer"))
            .await
            .expect("peer endpoint should respond");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert_eq!(response.text().await.unwrap(), "127.0.0.1");

        let _ = shutdown_tx.send(());
        server
            .await
            .expect("server task should join")
            .expect("server should shut down cleanly");
    }
}

#[cfg(not(feature = "server"))]
fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // dx serve rewrites Host before proxying to Axum. Hydrate the configured
    // local origin, not a forwarded header, so the browser can converge too.
    let local_origin = use_server_cached(auth::local_origin::from_fullstack);
    provide_context(local_origin.clone());
    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        if let (Some(canonical), Some(window)) = (&local_origin.0, web_sys::window()) {
            let location = window.location();
            if let Ok(current) = location.href() {
                if let Some(target) =
                    auth::local_origin::canonical_local_navigation_target(canonical, &current)
                {
                    let _ = location.replace(&target);
                }
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = &local_origin;
    });
    // Provide AuthContext as a signal at the root of the component tree so it's available
    // on both server (SSR) and client (hydration + client-side navigation).
    // We use a signal so we can update the context after fetching user profile.
    let initial_auth_context = use_server_cached(build_context_from_fullstack);
    let auth_context = use_signal(move || initial_auth_context);
    provide_context(auth_context);

    rsx! {
        // Document head
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: COLLECTIONS_CSS }
        document::Link { rel: "stylesheet", href: COLLECTION_FORMS_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        document::Link { rel: "stylesheet", href: MOBILE_CSS }
        document::Style { {format!("@font-face{{font-family:Fredoka;font-style:normal;font-weight:300 700;font-display:swap;src:url('{}') format('truetype')}}@font-face{{font-family:Nunito;font-style:normal;font-weight:200 1000;font-display:swap;src:url('{}') format('truetype')}}", FREDOKA, NUNITO)} }
        document::Meta { name: "viewport", content: "width=device-width, initial-scale=1, viewport-fit=cover" }

        // Error boundary wrapping all routes
        ErrorBoundary {
            handle_error: move |error: ErrorContext| {
                rsx! {
                    ErrorFallback { error }
                }
            },
            Router::<Route> {}
        }
    }
}

#[cfg(test)]
mod marketing_route_tests {
    use super::Route;

    #[test]
    fn marketing_routes_have_stable_public_paths() {
        assert_eq!(Route::Home {}.to_string(), "/");
        assert_eq!(Route::About {}.to_string(), "/about");
        assert_eq!(Route::HowTo {}.to_string(), "/how-to");
    }
}
