use dioxus::prelude::*;
#[cfg(feature = "server")]
use dioxus::server::{DioxusRouterExt, ServeConfig};

mod auth;
mod components;
#[cfg(feature = "server")]
mod db;
#[cfg(feature = "server")]
mod middleware;
mod pages;
#[cfg(all(feature = "server", test))]
mod test_utils;
mod utils;

use auth::context::{build_context_from_fullstack, AuthContext};
use components::{AppLayout, ErrorFallback};
use pages::{
    CollectionDetail, CollectionList, Dashboard, Explore, Home, Login, NotFound, RecipeDetail,
    RecipeNew, SettingsAccounts, SettingsProfile,
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
        RecipeDetail { id: i32 },
        #[route("/collections")]
        CollectionList {},
        #[route("/collections/:id")]
        CollectionDetail { id: i32 },
        #[route("/explore")]
        Explore {},
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
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

const GOOGLE_FONTS: &str = "https://fonts.googleapis.com/css2?family=Fredoka:wght@500;600;700&family=Nunito:wght@400;500;600;700&display=swap";

#[cfg(feature = "server")]
fn main() {
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let (google_client, github_client) = auth::oauth::build_oauth_clients(&base_url);

    // Build a custom Axum Router with OAuth routes and the Dioxus application.
    // The Dioxus router handles all non-API routes (SSR), and our OAuth routes
    // handle /auth/{provider}/start and /auth/{provider}/callback.
    // Auth middleware protects routes and injects user into request extensions.
    dioxus::server::serve(move || {
        let google_client = google_client.clone();
        let github_client = github_client.clone();
        async move {
            // Initialize pool lazily on Dioxus's runtime
            db::init_pool().await;
            let pool = db::get_pool();

            let rate_limit = middleware::rate_limit::RateLimitState::default();

            let state = auth::oauth::AppState {
                pool: pool.clone(),
                google_client,
                github_client,
                http_client: reqwest::Client::new(),
                rate_limit: rate_limit.clone(),
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
                .with_state(auth::user_profile::UserProfileState { pool: pool.clone() })
                .serve_dioxus_application(ServeConfig::new(), App);

            let oauth_router = axum::Router::new()
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
                    axum::routing::get(auth::logout::handle_logout)
                        .post(auth::logout::handle_logout),
                )
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    middleware::rate_limit::rate_limit_middleware,
                ))
                .with_state(state);

            // Spawn background cleanup task for rate limit state.
            // Runs every 60 seconds, removing stale entries to prevent memory growth.
            // Dropped on server shutdown.
            {
                let rl = rate_limit;
                tokio::spawn(async move {
                    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
                    loop {
                        interval.tick().await;
                        rl.cleanup();
                    }
                });
            }

            Ok(oauth_router.merge(dioxus_router))
        }
    });
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
