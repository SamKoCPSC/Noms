// Shared utilities.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub mod recipe_scaler;
pub mod reorder;
pub mod theme;

#[cfg(any(target_arch = "wasm32", test))]
pub mod image_processing_lifecycle;

#[cfg(target_arch = "wasm32")]
pub mod image_processing_client;
