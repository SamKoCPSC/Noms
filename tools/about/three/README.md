# First-kitchen Three.js prototype

Only stop 0 uses this renderer. Rust still owns the tour, camera matrices,
responsive framing, scrolling, loading curtain and lifecycle. Other stops use
the existing renderer. The kitchen loads fully assembled; static shadows travel
with the model instead of preceding animated furniture.

`kitchen.js` loads the embedded GLB with Three.js, presents the two baked color
atlases without applying lighting or tone mapping a second time, and retains
PBR glass and metal with a reflection environment made from the room and source
softboxes. It has no animation loop, mixer, dynamic shadows or per-frame
environment capture. Rust calls `draw` only when the tour needs a frame.

The transparent Three canvas fills the same viewport as the existing canvas,
behind the text overlay. Both use the same Rust model and camera transforms.
Failed asset loads, unavailable WebGL2, or a lost Three context fall back to the
existing kitchen. `?kitchen=legacy` explicitly selects it for comparison.

## Build

```sh
npm ci
npm run about:three:build
```

Commit the generated `assets/about-three-kitchen.js` with source changes. The
bundle is self-hosted; no CDN or runtime npm dependency is required. Three.js is
pinned in the lockfile; its MIT license is in `THREE-LICENSE.txt`.

The Blender master and bake instructions live in
[`assets/models/terracotta-recipe-kitchen`](../../../assets/models/terracotta-recipe-kitchen/README.md).

## Checks

```sh
python scripts/tests/test-baked-kitchen.py
cargo build --bin noms --target wasm32-unknown-unknown --no-default-features --features web
python scripts/tests/test-about-three-kitchen-browser.py --url http://127.0.0.1:8765/about
```

The browser check needs a running local app or a preview compiled from the
actual About page and tour modules, Playwright and Chromium. It covers first
load, resize, travel to another model, reduced motion, context loss and asset
failure.
The software-WebGL harness extends only the loader deadline from 25 to 120
seconds for GPU warm-up. Production retains its 25-second reading fallback;
this check is functional coverage, not a mobile performance benchmark.

## Limits

Opaque colors, texture detail and stationary shadows are baked from Blender.
Glass and metal remain view dependent; their browser reflections are an
approximation of the Blender lighting, not a pixel-identical Cycles render.
Two 2048px atlases trade close-up resolution for download and GPU memory. The
prototype retains the legacy kitchen asset for fallback and adds a second
WebGL context. Measure on target phones before extending it to every stop.
