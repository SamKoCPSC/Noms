use crate::components::base::PageIconKind;
use dioxus::prelude::*;

const MODELS: Asset = asset!("/assets/brand/icons-3d/drawer", AssetOptions::folder());
pub(super) const BOOTSTRAP: Asset = asset!(
    "/assets/drawer-models-bootstrap.js",
    JsAssetOptions::new().with_minify(false)
);
pub(super) const RENDERER: Asset = asset!(
    "/assets/home-kitchen.js",
    JsAssetOptions::new().with_minify(false)
);

#[component]
pub(super) fn DrawerArtwork(kind: Option<PageIconKind>) -> Element {
    let name = kind.map_or("noms-logo", PageIconKind::artwork_name);
    let height = if name == "noms-logo" { 40 } else { 32 };
    rsx! {
        span {
            class: "navigation-artwork drawer-model-icon",
            aria_hidden: "true",
            "data-icon": name,
            "data-model": "{MODELS}/{name}.glb",
            img { src: "{MODELS}/{name}.webp", alt: "", width: 32, height }
            canvas { width: 96, height: height * 3 }
        }
    }
}
