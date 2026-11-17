//! Landing-page enhancement: a real image remains usable before hydration or if 3D fails.
use dioxus::prelude::*;

const POSTER: Asset = asset!("/assets/landing/kitchen/poster.webp");
const MODEL: Asset = asset!("/assets/landing/kitchen/penguini-kitchen.glb");
const LIGHTING: Asset = asset!("/assets/landing/kitchen/lighting.json");
const RENDERER: Asset = asset!(
    "/assets/home-kitchen.js",
    JsAssetOptions::new().with_minify(false)
);
const _: &[u8] = include_bytes!("../../assets/home-kitchen.js");

#[component]
pub(super) fn HomeKitchen() -> Element {
    let mut paused = use_signal(|| true);
    #[allow(unused_mut)]
    let mut ready = use_signal(|| false);
    #[cfg(target_arch = "wasm32")]
    let runtime =
        use_hook(|| std::rc::Rc::new(std::cell::RefCell::new(browser::Runtime::default())));
    #[cfg(target_arch = "wasm32")]
    {
        let runtime = runtime.clone();
        use_drop(move || runtime.borrow_mut().dispose());
    }
    #[cfg(target_arch = "wasm32")]
    {
        let runtime = runtime.clone();
        use_effect(move || {
            let (is_ready, is_paused) = (ready(), paused());
            if is_ready {
                runtime.borrow().pause(is_paused);
            }
        });
    }
    rsx! {
        figure {
            id: "landing-kitchen", class: "landing-kitchen", "data-state": "loading",
            "data-model-url": MODEL, "data-lighting-url": LIGHTING, "data-renderer-url": RENDERER,
            onmounted: move |_| {
                #[cfg(target_arch = "wasm32")]
                {
                    let runtime = runtime.clone();
                    spawn(async move {
                        let Some(window) = web_sys::window() else { return };
                        let reduced = window.match_media("(prefers-reduced-motion: reduce)")
                            .ok().flatten().is_some_and(|m| m.matches());
                        paused.set(reduced);
                        if browser::mount(runtime.clone(), paused).await.is_ok() && !runtime.borrow().disposed {
                            ready.set(true);
                        }
                    });
                }
            },
            div { class: "landing-kitchen__scene", role: "img", aria_label: "Penguini bakes cookies in a coral and seafoam kitchen, mixing ingredients and reading his recipe book.",
                img { class: "landing-kitchen__poster", src: POSTER, alt: "", width: "800", height: "720", fetchpriority: "high" }
                canvas { id: "landing-kitchen-canvas", aria_hidden: "true" }
            }
            if ready() {
                button {
                    class: "landing-kitchen__pause", r#type: "button",
                    aria_label: if paused() { "Play animation" } else { "Pause animation" },
                    title: if paused() { "Play animation" } else { "Pause animation" },
                    aria_controls: "landing-kitchen-canvas", aria_pressed: paused(),
                    onclick: move |_| paused.toggle(),
                    svg { view_box: "0 0 24 24", width: "20", height: "20", fill: "currentColor", "aria-hidden": "true", "focusable": "false",
                        if paused() {
                            path { d: "M8 5.5v13l10-6.5z" }
                        } else {
                            path { d: "M6 5h4v14H6zM14 5h4v14h-4z" }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod browser {
    use dioxus::prelude::WritableExt;
    use std::{cell::RefCell, rc::Rc};
    use wasm_bindgen::{prelude::*, JsCast};
    use web_sys::{AbortController, AbortSignal, HtmlCanvasElement, MediaQueryList};

    #[wasm_bindgen(module = "/assets/home-kitchen-bootstrap.js")]
    extern "C" {
        #[wasm_bindgen(catch, js_name = loadKitchen)]
        async fn load(
            canvas: &HtmlCanvasElement,
            model: &str,
            lighting: &str,
            bridge: &str,
            signal: &AbortSignal,
        ) -> Result<JsValue, JsValue>;
        #[wasm_bindgen(catch, js_name = pauseKitchen)]
        fn set_paused(view: &JsValue, paused: bool) -> Result<(), JsValue>;
        #[wasm_bindgen(catch, js_name = disposeKitchen)]
        fn release(view: &JsValue) -> Result<(), JsValue>;
    }
    #[derive(Default)]
    pub(super) struct Runtime {
        view: Option<JsValue>,
        controller: Option<AbortController>,
        pub disposed: bool,
        motion_listener: Option<(MediaQueryList, Closure<dyn FnMut()>)>,
    }
    impl Runtime {
        pub fn pause(&self, paused: bool) {
            if let Some(view) = &self.view {
                let _ = set_paused(view, paused);
            }
        }
        pub fn dispose(&mut self) {
            self.disposed = true;
            if let Some((query, callback)) = self.motion_listener.take() {
                let _ = query.remove_event_listener_with_callback(
                    "change",
                    callback.as_ref().unchecked_ref(),
                );
            }
            if let Some(controller) = self.controller.take() {
                controller.abort();
            }
            if let Some(view) = self.view.take() {
                let _ = release(&view);
            }
        }
    }
    pub(super) async fn mount(
        runtime: Rc<RefCell<Runtime>>,
        mut paused: dioxus::prelude::Signal<bool>,
    ) -> Result<(), JsValue> {
        if let Some(query) = web_sys::window().and_then(|w| {
            w.match_media("(prefers-reduced-motion: reduce)")
                .ok()
                .flatten()
        }) {
            let observed = query.clone();
            let callback = Closure::<dyn FnMut()>::new(move || {
                if observed.matches() {
                    paused.set(true);
                }
            });
            query.add_event_listener_with_callback("change", callback.as_ref().unchecked_ref())?;
            runtime.borrow_mut().motion_listener = Some((query, callback));
        }
        let doc = web_sys::window()
            .and_then(|w| w.document())
            .ok_or("No document")?;
        let host = doc
            .get_element_by_id("landing-kitchen")
            .ok_or("Missing kitchen")?;
        let canvas = doc
            .get_element_by_id("landing-kitchen-canvas")
            .ok_or("Missing canvas")?
            .dyn_into::<HtmlCanvasElement>()?;
        let url = |name| {
            host.get_attribute(name)
                .ok_or_else(|| JsValue::from_str("Missing kitchen asset"))
        };
        let controller = AbortController::new()?;
        let signal = controller.signal();
        runtime.borrow_mut().controller = Some(controller);
        match load(
            &canvas,
            &url("data-model-url")?,
            &url("data-lighting-url")?,
            &url("data-renderer-url")?,
            &signal,
        )
        .await
        {
            Ok(view) => {
                let mut state = runtime.borrow_mut();
                if state.disposed {
                    let _ = release(&view);
                } else {
                    state.view = Some(view);
                }
                Ok(())
            }
            Err(error) => {
                if !runtime.borrow().disposed {
                    let _ = host.set_attribute("data-state", "fallback");
                }
                Err(error)
            }
        }
    }
}
