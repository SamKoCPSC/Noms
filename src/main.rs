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

use components::{AppLayout, ErrorFallback};
use pages::{
    CollectionDetail, CollectionList, Dashboard, Explore, Home, Login, RecipeDetail, RecipeNew,
    SettingsAccounts, SettingsProfile,
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
        #[route("/settings/profile")]
        SettingsProfile {},
        #[route("/settings/accounts")]
        SettingsAccounts {},
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

const GOOGLE_FONTS: &str = "https://fonts.googleapis.com/css2?family=Fredoka:wght@500;600;700&family=Nunito:wght@400;500;600;700&display=swap";

#[cfg(feature = "server")]
fn main() {
    // Create and validate the database connection pool.
    // Uses a dedicated thread with its own runtime to avoid conflicting
    // with Dioxus's own runtime management.
    let pool = std::thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime");
        rt.block_on(db::create_pool())
    })
    .join()
    .expect("database initialization thread panicked")
    .expect("Failed to create database pool");

    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let (google_client, github_client) = auth::oauth::build_oauth_clients(&base_url);

    let state = auth::oauth::AppState {
        pool,
        google_client,
        github_client,
        http_client: reqwest::Client::new(),
    };

    // Build a custom Axum Router with OAuth routes and the Dioxus application.
    // The Dioxus router handles all non-API routes (SSR), and our OAuth routes
    // handle /auth/{provider}/start and /auth/{provider}/callback.
    // Auth middleware protects routes and injects user into request extensions.
    dioxus::server::serve(move || {
        let state = state.clone();
        async move {
            let dioxus_router = axum::Router::new()
                .layer(axum::middleware::from_fn(middleware::auth::handle_auth))
                .serve_dioxus_application(
                    ServeConfig::new()
                        .context_provider(auth::context::build_context_from_fullstack),
                    App,
                );

            let oauth_router = axum::Router::new()
                .route(
                    "/auth/{provider}/start",
                    axum::routing::get(auth::oauth::start_handler),
                )
                .route(
                    "/auth/{provider}/callback",
                    axum::routing::get(auth::oauth::callback_handler),
                )
                .with_state(state);

            Ok(dioxus_router.merge(oauth_router))
        }
    });
}

#[cfg(not(feature = "server"))]
fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
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
