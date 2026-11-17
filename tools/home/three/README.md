# Landing-page kitchen renderer

`HomeKitchen` owns loading, pause state, reduced-motion preference and route
cleanup through a narrow wasm-bindgen bridge. This bundled Three.js module owns
the animation mixer, camera fitting and render scheduling. It shares the About
renderer’s display-baked lighting and GPU cleanup helpers; Three and Meshopt are
pinned in the existing package lock, bundled locally, and loaded only on Home.

The entire 45.2-second clip repeats indefinitely. A short fade masks the reset of
ingredients. Pausing always shows a fully opaque still. Reduced motion starts
paused and changing that preference pauses playback. Users can explicitly Play.
Offscreen/background rendering suspends without advancing the animation clock;
pixel density is capped at 1.5. Navigation aborts pending downloads and releases
mixers, skeletons, textures, geometry, observers, listeners and the WebGL context.
An ordinary poster remains available while loading, without JavaScript, on asset
failure or after context loss. The image does not block search or other content.

```sh
npm ci
npm run home:three:build
npm run about:three:build  # when shared scene-resources.js changes
cargo build --locked --bin noms --target wasm32-unknown-unknown --no-default-features --features web
python scripts/tests/test-home-kitchen-browser.py --url http://127.0.0.1:8765/ --output /tmp/home-kitchen-checks
```

Commit both generated renderer bundles after editing shared code. See the Three
and Meshopt licenses in `tools/about/three/`. The browser test covers the actual
page, two loops, controls, responsive layout, reduced motion, offscreen suspension,
route cleanup and fallbacks. Software WebGL checks correctness, not phone GPU
performance. Model source and bake instructions live in
`assets/models/penguini-cookie-kitchen/README.md`.
