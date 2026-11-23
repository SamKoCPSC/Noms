//! Three.js rendering adapter. All camera and navigation state stays in Rust.
use super::{flight::Camera, math::Mat4};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, HtmlCanvasElement, HtmlElement};

#[wasm_bindgen(module = "/assets/about-three-bootstrap.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = loadTour)]
    async fn load_tour(
        canvas: &HtmlCanvasElement,
        assets: &str,
        initial: usize,
        bridge: &str,
    ) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch, js_name = beginTour)]
    fn begin_tour(view: &JsValue, progress: f32, width: u32, height: u32) -> Result<bool, JsValue>;
    #[wasm_bindgen(catch, js_name = hasTourScene)]
    fn has_scene(view: &JsValue, index: usize) -> Result<bool, JsValue>;
    #[wasm_bindgen(catch, js_name = drawTour)]
    fn draw_tour(
        view: &JsValue,
        index: usize,
        model: &[f32],
        vp: &[f32],
        camera_view: &[f32],
        visibility: f32,
    ) -> Result<bool, JsValue>;
    #[wasm_bindgen(catch, js_name = hideTour)]
    fn hide_tour(view: &JsValue) -> Result<(), JsValue>;
    #[wasm_bindgen(catch, js_name = disposeTour)]
    fn dispose_tour(view: &JsValue) -> Result<(), JsValue>;
}

pub struct ThreeTour {
    view: JsValue,
}
impl ThreeTour {
    pub async fn load(doc: &Document, root: &HtmlElement, initial: usize) -> Result<Self, JsValue> {
        let url = |key: &str| {
            root.get_attribute(key)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| JsValue::from_str("Missing Three tour asset"))
        };
        let canvas = doc
            .get_element_by_id("story-three-canvas")
            .ok_or_else(|| JsValue::from_str("Missing Three canvas"))?
            .dyn_into::<HtmlCanvasElement>()?;
        let assets = crate::pages::about::MODEL_FOR_STOP.iter().map(|asset| {
            let name = crate::pages::about::BAKED_MODEL_NAMES[*asset];
            Ok(serde_json::json!({"model": url(&format!("data-baked-{name}-url"))?, "lighting": url(&format!("data-baked-{name}-lighting"))?}))
        }).collect::<Result<Vec<_>, JsValue>>()?;
        let view = load_tour(
            &canvas,
            &serde_json::to_string(&assets)
                .map_err(|_| JsValue::from_str("Invalid baked asset catalog"))?,
            initial,
            &url("data-three-bridge-url")?,
        )
        .await?;
        Ok(Self { view })
    }
    pub fn begin(&self, progress: f32, width: u32, height: u32) -> bool {
        begin_tour(&self.view, progress, width, height).unwrap_or(false)
    }
    pub fn has(&self, index: usize) -> bool {
        has_scene(&self.view, index).unwrap_or(false)
    }
    pub fn draw(&self, index: usize, model: Mat4, camera: &Camera, visibility: f32) -> bool {
        draw_tour(
            &self.view,
            index,
            &model,
            &camera.vp,
            &camera.view,
            visibility,
        )
        .unwrap_or(false)
    }
    pub fn hide(&self) {
        let _ = hide_tour(&self.view);
    }
}
impl Drop for ThreeTour {
    fn drop(&mut self) {
        let _ = dispose_tour(&self.view);
    }
}
