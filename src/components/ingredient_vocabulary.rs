//! One reactive index per browser app, persisted as public catalog data only.
use dioxus::prelude::*;
use noms::vocabulary::index::SearchIndex;
use std::sync::Arc;

#[derive(Default)]
pub(crate) struct ClientState {
    pub index: Option<Arc<SearchIndex>>,
    #[cfg(target_arch = "wasm32")]
    popularity: Vec<(uuid::Uuid, i64)>,
    #[cfg(target_arch = "wasm32")]
    checking: bool,
    #[cfg(target_arch = "wasm32")]
    storage_loaded: bool,
    #[cfg(target_arch = "wasm32")]
    last_attempt: Option<f64>,
    #[cfg(target_arch = "wasm32")]
    retry: bool,
}

// Dioxus globals are scoped to the VirtualDom, never shared across SSR requests.
static VOCABULARY: GlobalSignal<ClientState> = Signal::global(ClientState::default);

pub(crate) fn state() -> Signal<ClientState> {
    VOCABULARY.signal()
}

/// Called once by each editor, not by each ingredient row.
pub(crate) fn use_editor_vocabulary() {
    let shared = state();
    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        browser::refresh(shared, true);
        #[cfg(not(target_arch = "wasm32"))]
        let _ = shared;
    });
    use_hook(move || {
        #[cfg(target_arch = "wasm32")]
        {
            browser::listen(shared)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = shared;
            std::rc::Rc::new(())
        }
    });
}

pub(crate) fn on_focus(shared: Signal<ClientState>) {
    #[cfg(target_arch = "wasm32")]
    browser::refresh(shared, false);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = shared;
}

#[cfg(target_arch = "wasm32")]
mod browser {
    use super::*;
    use noms::vocabulary::index::{CachedIndex, Catalog, Manifest, MANIFEST_PATH, MAX_INDEX_BYTES};
    use wasm_bindgen::{closure::Closure, JsCast};
    use wasm_bindgen_futures::JsFuture;

    const CACHE_NAME: &str = "noms-ingredient-vocabulary-v1";
    const CACHE_KEY: &str = "/__noms_cache/ingredient-vocabulary-v1";

    async fn cache() -> Option<web_sys::Cache> {
        let storage = web_sys::window()?.caches().ok()?;
        JsFuture::from(storage.open(CACHE_NAME))
            .await
            .ok()?
            .dyn_into()
            .ok()
    }

    async fn cached() -> Option<CachedIndex> {
        let cache = cache().await?;
        let response: web_sys::Response = JsFuture::from(cache.match_with_str(CACHE_KEY))
            .await
            .ok()?
            .dyn_into()
            .ok()?;
        let text = JsFuture::from(response.text().ok()?)
            .await
            .ok()?
            .as_string()?;
        if text.len() > MAX_INDEX_BYTES + 8 * 1024 * 1024 {
            return None;
        }
        serde_json::from_str(&text).ok()
    }

    async fn persist(index: &SearchIndex, popularity: &[(uuid::Uuid, i64)]) {
        let Some(cache) = cache().await else {
            return;
        };
        #[derive(serde::Serialize)]
        struct Stored<'a> {
            version: &'a str,
            catalog: &'a Catalog,
            popularity: &'a [(uuid::Uuid, i64)],
        }
        let Ok(text) = serde_json::to_string(&Stored {
            version: index.version(),
            catalog: index.catalog(),
            popularity,
        }) else {
            return;
        };
        let Ok(response) = web_sys::Response::new_with_opt_str(Some(&text)) else {
            return;
        };
        let _ = JsFuture::from(cache.put_with_str(CACHE_KEY, &response)).await;
    }

    async fn fetch(url: &str, manifest: bool) -> Result<String, u16> {
        let controller = web_sys::AbortController::new().map_err(|_| 0u16)?;
        let cancel = controller.clone();
        let _timer = gloo_timers::callback::Timeout::new(
            if manifest { 10_000 } else { 30_000 },
            move || cancel.abort(),
        );
        let response = gloo_net::http::Request::get(url)
            .cache(if manifest {
                web_sys::RequestCache::NoCache
            } else {
                web_sys::RequestCache::Default
            })
            .abort_signal(Some(&controller.signal()))
            .send()
            .await
            .map_err(|_| 0u16)?;
        if response.status() != 200 {
            return Err(response.status());
        }
        let text = response.text().await.map_err(|_| 0u16)?;
        if text.len()
            > if manifest {
                8 * 1024 * 1024
            } else {
                MAX_INDEX_BYTES
            }
        {
            return Err(0);
        }
        Ok(text)
    }

    pub(super) fn refresh(mut shared: Signal<ClientState>, force: bool) {
        let now = js_sys::Date::now();
        {
            let current = shared.peek();
            let interval = if current.retry { 5_000.0 } else { 60_000.0 };
            if current.checking
                || (!force
                    && current
                        .last_attempt
                        .is_some_and(|last| now - last < interval))
            {
                return;
            }
        }
        shared.write().checking = true;
        shared.write().last_attempt = Some(now);
        // This task belongs to the app root, so navigating away mid-download
        // cannot leave a shared "checking" flag stuck for the next editor.
        dioxus::core::spawn_forever(async move {
            let load_storage = !shared.peek().storage_loaded;
            shared.write().storage_loaded = true;
            if load_storage {
                if let Some(old) = cached().await {
                    if let Ok(index) = SearchIndex::new(&old.version, old.catalog, &old.popularity)
                    {
                        let mut current = shared.write();
                        if current.index.is_none() {
                            current.index = Some(Arc::new(index));
                            current.popularity = old.popularity;
                        }
                    }
                }
            }
            let result = refresh_inner(shared).await;
            let mut current = shared.write();
            current.checking = false;
            current.retry = result.is_err();
        });
    }

    async fn refresh_inner(mut shared: Signal<ClientState>) -> Result<(), ()> {
        for attempt in 0..2 {
            let text = fetch(MANIFEST_PATH, true).await.map_err(|_| ())?;
            let manifest: Manifest = serde_json::from_str(&text).map_err(|_| ())?;
            manifest.validate().map_err(|_| ())?;
            let old = shared.peek().index.clone();
            let changed = old.as_ref().is_none_or(|i| i.version() != manifest.version);
            let index = if !changed {
                if shared.peek().popularity == manifest.popularity {
                    return Ok(());
                }
                old.unwrap()
                    .with_popularity(&manifest.popularity)
                    .map_err(|_| ())?
            } else {
                match fetch(&manifest.url, false).await {
                    Ok(text) => {
                        let catalog: Catalog = serde_json::from_str(&text).map_err(|_| ())?;
                        SearchIndex::new(&manifest.version, catalog, &manifest.popularity)
                            .map_err(|_| ())?
                    }
                    // A deploy may commit between the manifest and content GET.
                    Err(410) if attempt == 0 => {
                        gloo_timers::future::TimeoutFuture::new(1100).await;
                        continue;
                    }
                    Err(_) => return Err(()),
                }
            };
            let index = Arc::new(index);
            {
                let mut current = shared.write();
                current.index = Some(index.clone());
                current.popularity = manifest.popularity.clone();
            }
            persist(&index, &manifest.popularity).await;
            return Ok(());
        }
        Err(())
    }

    pub(super) struct FocusListener {
        callback: Closure<dyn FnMut(web_sys::Event)>,
    }
    pub(super) fn listen(shared: Signal<ClientState>) -> std::rc::Rc<FocusListener> {
        let callback = Closure::wrap(Box::new(move |_: web_sys::Event| {
            if web_sys::window()
                .and_then(|w| w.document())
                .is_some_and(|d| !d.hidden())
            {
                refresh(shared, false);
            }
        }) as Box<dyn FnMut(web_sys::Event)>);
        if let Some(window) = web_sys::window() {
            let _ =
                window.add_event_listener_with_callback("focus", callback.as_ref().unchecked_ref());
            if let Some(document) = window.document() {
                let _ = document.add_event_listener_with_callback(
                    "visibilitychange",
                    callback.as_ref().unchecked_ref(),
                );
            }
        }
        std::rc::Rc::new(FocusListener { callback })
    }
    impl Drop for FocusListener {
        fn drop(&mut self) {
            if let Some(window) = web_sys::window() {
                let _ = window.remove_event_listener_with_callback(
                    "focus",
                    self.callback.as_ref().unchecked_ref(),
                );
                if let Some(document) = window.document() {
                    let _ = document.remove_event_listener_with_callback(
                        "visibilitychange",
                        self.callback.as_ref().unchecked_ref(),
                    );
                }
            }
        }
    }
}
