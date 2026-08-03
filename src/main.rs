use dioxus::prelude::*;
#[cfg(feature = "server")]
use dioxus::server::{DioxusRouterExt, ServeConfig};
use manganis::CssAssetOptions;

mod api;
mod auth;
mod components;
#[cfg(feature = "server")]
mod db;
#[cfg(feature = "server")]
mod middleware;
mod pages;
mod search;
mod search_url;
#[cfg(feature = "server")]
mod storage;
#[cfg(all(feature = "server", test))]
mod test_utils;
mod types;
mod utils;
#[cfg(feature = "server")]
mod worker;

use auth::context::build_context_from_fullstack;
#[cfg(target_arch = "wasm32")]
use auth::context::AuthContext;
use components::{AppLayout, ErrorFallback};
use pages::{
    CollectionDetail, CollectionEdit, CollectionList, CollectionNew, Dashboard, Explore, Home,
    Login, NotFound, RecipeDetail, RecipeDraftEdit, RecipeEdit, RecipeNew, RecipeVersionDetail,
    RecipeVersionGraph, Search, SettingsAccounts, SettingsProfile, UserProfile,
};

/// Application routes.
#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(AppLayout)]
        #[route("/")]
        Home {},
        #[route("/login")]
        Login {},
        #[route("/dashboard")]
        Dashboard {},
        #[route("/recipes/new")]
        RecipeNew {},
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
        #[route("/collections/new")]
        CollectionNew {},
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
        #[redirect("/settings", || Route::SettingsProfile {})]
        #[route("/settings/profile")]
        #[route("/settings/profile")]
        SettingsProfile {},
        #[route("/settings/accounts")]
        SettingsAccounts {},
        // Catch-all: matches any route not defined above
        #[route("/:..segments")]
        NotFound { segments: Vec<String> },
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

const GOOGLE_FONTS: &str = "https://fonts.googleapis.com/css2?family=Fredoka:wght@500;600;700&family=Nunito:wght@400;500;600;700&display=swap";

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
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let (google_client, github_client) = auth::oauth::build_oauth_clients(&base_url);

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
        rate_limit: rate_limit.clone(),
        storage: storage_client.clone(),
    };

    let dioxus_router = axum::Router::new()
        .layer(axum::Extension(pool.clone()))
        .layer(axum::middleware::from_fn_with_state(
            pool.clone(),
            middleware::auth::handle_auth,
        ))
        .route(
            "/api/user_profile",
            axum::routing::get(auth::user_profile::handle_user_profile),
        )
        .route(
            "/api/upload",
            axum::routing::post(api::upload::upload_handler),
        )
        .layer(axum::Extension(storage_client))
        .with_state(auth::user_profile::UserProfileState { pool })
        .serve_dioxus_application(ServeConfig::new(), App)
        .layer(api::upload::server_function_body_limit_layer());

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

    oauth_router.merge(dioxus_router)
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
            axum::routing::get(auth::oauth::callback_handler),
        )
        .route(
            "/auth/logout",
            axum::routing::get(auth::logout::handle_logout).post(auth::logout::handle_logout),
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

    use axum::extract::ConnectInfo;
    use axum::routing::get;
    use tokio::sync::oneshot;

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
    // Provide AuthContext as a signal at the root of the component tree so it's available
    // on both server (SSR) and client (hydration + client-side navigation).
    // We use a signal so we can update the context after fetching user profile.
    let auth_context = use_signal(build_context_from_fullstack);
    provide_context(auth_context);

    // On client, fetch user profile after hydration to update AuthContext
    #[cfg(target_arch = "wasm32")]
    {
        use_hook(move || {
            let mut ctx = auth_context;
            spawn(async move {
                let res = gloo_net::http::Request::get("/api/user_profile")
                    .send()
                    .await;
                if let Ok(response) = res {
                    if response.ok() {
                        let body = response.text().await.unwrap_or_default();
                        if let Ok(user_ctx) = serde_json::from_str::<AuthContext>(&body) {
                            ctx.set(user_ctx);
                        }
                    }
                }
            });
        });
    }

    rsx! {
        // Document head
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        document::Link { rel: "preconnect", href: "https://fonts.googleapis.com" }
        document::Link { rel: "preconnect", href: "https://fonts.gstatic.com", crossorigin: "anonymous" }
        document::Link { rel: "stylesheet", href: GOOGLE_FONTS }
        document::Meta { name: "viewport", content: "width=device-width, initial-scale=1" }

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
