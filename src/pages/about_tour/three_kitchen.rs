//! Three.js rendering adapter. All camera and navigation state stays in Rust.
use super::{flight::Camera, math::Mat4};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, HtmlCanvasElement, HtmlElement};

#[wasm_bindgen(module = "/assets/about-three-bootstrap.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = loadKitchen)]
    async fn load_kitchen(
        canvas: &HtmlCanvasElement,
        model: &str,
        lighting: &str,
        bridge: &str,
    ) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch, js_name = drawKitchen)]
    fn draw_kitchen(
        view: &JsValue,
        model: &[f32],
        vp: &[f32],
        camera_view: &[f32],
        width: u32,
        height: u32,
        visibility: f32,
    ) -> Result<bool, JsValue>;
    #[wasm_bindgen(catch, js_name = hideKitchen)]
    fn hide_kitchen(view: &JsValue) -> Result<(), JsValue>;
    #[wasm_bindgen(catch, js_name = disposeKitchen)]
    fn dispose_kitchen(view: &JsValue) -> Result<(), JsValue>;
}

pub struct ThreeKitchen {
    view: JsValue,
}
impl ThreeKitchen {
    pub async fn load(doc: &Document, root: &HtmlElement) -> Result<Self, JsValue> {
        let url = |key: &str| {
            root.get_attribute(key)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| JsValue::from_str("Missing Three kitchen asset"))
        };
        let canvas = doc
            .get_element_by_id("story-kitchen-canvas")
            .ok_or_else(|| JsValue::from_str("Missing kitchen canvas"))?
            .dyn_into::<HtmlCanvasElement>()?;
        let view = load_kitchen(
            &canvas,
            &url("data-three-kitchen-url")?,
            &url("data-three-lighting-url")?,
            &url("data-three-bridge-url")?,
        )
        .await?;
        Ok(Self { view })
    }
    pub fn draw(
        &self,
        model: Mat4,
        camera: &Camera,
        width: u32,
        height: u32,
        visibility: f32,
    ) -> bool {
        draw_kitchen(
            &self.view,
            &model,
            &camera.vp,
            &camera.view,
            width,
            height,
            visibility,
        )
        .unwrap_or(false)
    }
    pub fn hide(&self) {
        let _ = hide_kitchen(&self.view);
    }
}
impl Drop for ThreeKitchen {
    fn drop(&mut self) {
        let _ = dispose_kitchen(&self.view);
    }
}
