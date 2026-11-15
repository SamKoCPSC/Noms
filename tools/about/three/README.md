# About-tour Three.js renderer

All twelve stops use the shared `tour.js` renderer. Rust owns navigation, camera
matrices, responsive framing, scrolling, loading curtain and lifecycle. Scenes
are fully assembled before display. They have baked shadows and no assembly
animation, animation mixer, dynamic shadow maps or independent RAF loop.

Blender display-baked opaque materials are unlit and bypass Three tone mapping.
Glass also bypasses the final tone map so transmitted display-baked colors are
not processed twice. Glass and metal remain view dependent, using one reflection environment per
resident scene, built from its baked room and recorded source softboxes. Rust
supplies the real view and projection matrices for correct PBR transmission.
Layered transmission, such as a drink inside a glass, and refracted display-case
contents remain approximate in Three's screen-space transmission pass.

One transparent canvas fills the viewport behind the text. Up to three baked
scenes (the current stop and its neighbors) are resident; at most two requests
are in flight. Traveling away disposes meshes, textures, materials and reflection
targets. Loading the current stop is part of the existing loading curtain;
neighbors load on demand. Completion events wake the paused Rust renderer.

The existing renderer is retained as a static fallback during a scene download,
for individual failed assets, unavailable WebGL2, or a lost Three context. Its
original assets are still loaded for framing and fallback. This bounds additional
Three resources, but does not eliminate the legacy renderer's memory footprint.
`?renderer=legacy` selects the comparison path (`?kitchen=legacy` also remains
accepted). Reduced motion and reading mode retain the existing still-image page.

## Build and checks

```sh
npm ci
npm run about:three:build
python scripts/tests/test-baked-tour.py
cargo build --bin noms --target wasm32-unknown-unknown --no-default-features --features web
python scripts/tests/test-about-three-tour-browser.py --url http://127.0.0.1:8765/about
```

Commit the generated `assets/about-three-tour.js` with source changes. Three is
pinned and self-hosted; no CDN is used. See `THREE-LICENSE.txt` and `MESHOPT-LICENSE.txt`. The matching Meshopt decoder
is bundled locally with the renderer.

The browser check runs the real Rust tour with Playwright/Chromium. It visits all
stops, revisits evicted scenes, checks bounded residency, initial/resized camera
proportions, static arrival frames, reduced motion, direct fragment entry,
context loss and per-scene asset failure. Software WebGL gets a test-only 120s
loader and asset deadlines; production keeps its 25s reading fallback and 20s
asset-download timeout. This is functional
coverage, not a physical-phone memory/frame-rate benchmark.

See [bake instructions](../baked/README.md) for source provenance and limitations.
The home office, bedroom and garden use reconstructed lighting because their
original Blender masters are not in the repository; all other bakes use their
archived source shaders and lighting. Glass/reflections remain approximations.
