#[cfg(any(target_arch = "wasm32", test))]
mod assembly;
#[cfg(target_arch = "wasm32")]
mod browser;
#[cfg(any(target_arch = "wasm32", test))]
mod flight;
#[cfg(any(target_arch = "wasm32", test))]
mod gltf;
#[cfg(any(target_arch = "wasm32", test))]
mod loading;
#[cfg(any(target_arch = "wasm32", test))]
mod math;
#[cfg(target_arch = "wasm32")]
mod scene;
#[cfg(target_arch = "wasm32")]
pub(super) use browser::{start, stop};

#[cfg(target_arch = "wasm32")]
pub(crate) use browser::capture_entry_fragment;

#[cfg(target_arch = "wasm32")]
mod shadow_map;
#[cfg(any(target_arch = "wasm32", test))]
mod shadows;
