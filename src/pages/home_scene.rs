//! Landing-page enhancement: a real image remains usable before hydration or if 3D fails.
use dioxus::prelude::*;

const KITCHEN_POSTER: Asset = asset!("/assets/landing/kitchen/poster.webp");
const KITCHEN_MODEL: Asset = asset!("/assets/landing/kitchen/penguini-kitchen.glb");
const DESK_POSTER: Asset = asset!("/assets/landing/recipe-desk/poster.webp");
const DESK_MODEL: Asset = asset!("/assets/landing/recipe-desk/recipe-desk.glb");
const BANQUET_POSTER: Asset = asset!("/assets/landing/pandacotta-banquet/poster.webp");
const BANQUET_MODEL: Asset = asset!("/assets/landing/pandacotta-banquet/pandacotta-banquet.glb");
const ISLAND_POSTER: Asset = asset!("/assets/landing/owlfredo-kitchen-island/poster.webp");
const ISLAND_MODEL: Asset =
    asset!("/assets/landing/owlfredo-kitchen-island/owlfredo-kitchen-island.glb");
const PLATING_POSTER: Asset = asset!(
    "/assets/landing/signature-recipe/poster.webp",
    manganis::AssetOptions::builder()
);
const PLATING_MODEL: Asset = asset!("/assets/landing/signature-recipe/terrimisu-and-owlfredo.glb");
const COOKIE_STUDY_POSTER: Asset = asset!(
    "/assets/landing/terrimisu-cookie-study/poster.webp",
    manganis::AssetOptions::builder()
);
const COOKIE_STUDY_MODEL: Asset =
    asset!("/assets/landing/terrimisu-cookie-study/terrimisu-cookie-study.glb");
const LIGHTING: Asset = asset!("/assets/landing/kitchen/lighting.json");
const RENDERER: Asset = asset!(
    "/assets/home-kitchen.js",
    JsAssetOptions::new().with_minify(false)
);
const _: &[u8] = include_bytes!("../../assets/home-kitchen.js");

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Scene {
    Kitchen,
    RecipeDesk,
    Banquet,
    SoupIsland,
    Plating,
    CookieStudy,
}

#[component]
pub(super) fn HomeScene(scene: Scene, caption: Option<&'static str>) -> Element {
    let (name, animation, model, poster, width, height, description) = match scene {
        Scene::Kitchen => ("kitchen", "kitchen animation", KITCHEN_MODEL, KITCHEN_POSTER, 800, 720,
            "Penguini browns butter while checking his recipe book. He pours it into an empty bowl on a kitchen scale, adds brown sugar, eggs, vanilla and baking soda, then opens a jar and shakes in espresso powder. He stirs, adds flour and chocolate chips, mixes again, and bakes three cookies. He eats one in five bites and does a happy dance."),
        Scene::RecipeDesk => ("recipe-desk", "recipe desk animation", DESK_MODEL, DESK_POSTER, 1200, 1000,
            "Penguini and Cattatouille work on a recipe at their laptops, pointing out changes and nodding in agreement."),
        Scene::Banquet => ("pandacotta-banquet", "banquet animation", BANQUET_MODEL, BANQUET_POSTER, 1120, 640,
            "Pandacotta lifts silver domes at a banquet table to discover bao, tacos, and strawberry cream cake. He tastes each dish."),
        Scene::SoupIsland => ("owlfredo-kitchen-island", "soup animation", ISLAND_MODEL, ISLAND_POSTER, 960, 720,
            "Owlfredo stirs, seasons, and tastes tomato soup at a sage-green kitchen island, then does two happy hops."),
        Scene::Plating => ("signature-recipe", "plating animation", PLATING_MODEL, PLATING_POSTER, 800, 662,
            "Terrimisu finishes the ravioli with sauce, pepper, grated truffle and herbs, wipes the plate, and serves Owlfredo. Owlfredo takes three bites and both chefs beam."),
        Scene::CookieStudy => ("terrimisu-cookie-study", "cookie tasting animation", COOKIE_STUDY_MODEL, COOKIE_STUDY_POSTER, 896, 784,
            "Terrimisu tastes cookies from two batches. He tilts his head and squints at the first, then hops with delight while holding the second."),
    };
    let canvas_id = format!("landing-{name}-canvas");
    let is_hero = scene == Scene::Kitchen;
    let mut paused = use_signal(|| true);
    #[allow(unused_mut)]
    let mut ready = use_signal(|| false);
    #[allow(unused_mut)]
    let mut manual_start = use_signal(|| false);
    #[cfg(target_arch = "wasm32")]
    let mut load_requested = use_signal(|| false);
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
    #[cfg(target_arch = "wasm32")]
    {
        let runtime = runtime.clone();
        use_effect(move || {
            if load_requested() {
                let runtime = runtime.clone();
                spawn(async move {
                    if browser::mount(runtime.clone(), paused, name).await.is_ok()
                        && !runtime.borrow().disposed
                    {
                        ready.set(true);
                    }
                });
            }
        });
    }
    let control = if paused() { "Play" } else { "Pause" };
    rsx! {
        figure {
            id: "landing-{name}", class: "landing-model", "data-state": "loading",
            "data-scene": name,
            "data-model-url": model, "data-renderer-url": RENDERER,
            "data-lighting-url": is_hero.then(|| LIGHTING.to_string()),
            onmounted: move |_| {
                #[cfg(target_arch = "wasm32")]
                {
                    let Some(window) = web_sys::window() else { return };
                    let reduced = window.match_media("(prefers-reduced-motion: reduce)")
                        .ok().flatten().is_some_and(|m| m.matches());
                    paused.set(reduced);
                    if reduced && matches!(scene, Scene::Plating | Scene::CookieStudy) {
                        manual_start.set(true);
                    } else {
                        load_requested.set(true);
                    }
                }
            },
            div { class: "landing-model__viewer",
                div { role: "img", aria_label: description,
                    img { class: "landing-model__poster", src: poster, alt: "", width: "{width}", height: "{height}",
                        loading: if is_hero { "eager" } else { "lazy" }, fetchpriority: if is_hero { "high" } else { "low" }
                    }
                    canvas { id: "{canvas_id}", aria_hidden: "true" }
                }
                if ready() || manual_start() {
                    button {
                        class: "landing-model__pause", r#type: "button",
                        aria_label: "{control} {animation}", title: "{control} {animation}",
                        aria_controls: "{canvas_id}", aria_pressed: paused(),
                        onclick: move |_| {
                            paused.toggle();
                            #[cfg(target_arch = "wasm32")]
                            if !load_requested() { load_requested.set(true); }
                        },
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
            if let Some(caption) = caption { figcaption { "{caption}" } }
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
        #[wasm_bindgen(catch, js_name = loadScene)]
        async fn load(canvas: &HtmlCanvasElement, signal: &AbortSignal)
            -> Result<JsValue, JsValue>;
        #[wasm_bindgen(catch, js_name = pauseScene)]
        fn set_paused(view: &JsValue, paused: bool) -> Result<(), JsValue>;
        #[wasm_bindgen(catch, js_name = disposeScene)]
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
        name: &str,
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
        let id = format!("landing-{name}");
        let host = doc.get_element_by_id(&id).ok_or("Missing landing scene")?;
        let canvas = doc
            .get_element_by_id(&format!("{id}-canvas"))
            .ok_or("Missing canvas")?
            .dyn_into::<HtmlCanvasElement>()?;
        let controller = AbortController::new()?;
        let signal = controller.signal();
        runtime.borrow_mut().controller = Some(controller);
        match load(&canvas, &signal).await {
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
