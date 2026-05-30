use dioxus::prelude::*;

mod auth;
mod components;
#[cfg(feature = "server")]
mod db;
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
    // Validate database connectivity before starting the server.
    // Uses a dedicated thread with its own runtime to avoid conflicting
    // with Dioxus's own runtime management.
    let result = std::thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime");
        rt.block_on(db::create_pool())
    })
    .join()
    .expect("database initialization thread panicked");

    if let Err(e) = result {
        eprintln!("Fatal: {e}");
        std::process::exit(1);
    }

    dioxus::launch(App);
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
