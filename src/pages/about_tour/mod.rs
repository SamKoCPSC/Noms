#[cfg(any(target_arch = "wasm32", test))]
mod assembly;
#[cfg(target_arch = "wasm32")]
mod browser;
#[cfg(target_arch = "wasm32")]
mod flight;
#[cfg(any(target_arch = "wasm32", test))]
mod gltf;
#[cfg(target_arch = "wasm32")]
mod math;
#[cfg(target_arch = "wasm32")]
mod scene;
#[cfg(target_arch = "wasm32")]
pub(super) use browser::{start, stop};
