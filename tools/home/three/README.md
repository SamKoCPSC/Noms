# Landing-page 3D scenes

`HomeScene` owns each model's poster, pause control, reduced-motion preference
and route cleanup. Both scenes share one locally bundled Three.js renderer. The
hero loads immediately. The recipe desk waits until the mission section is near
the viewport.

The hero's 45.2-second kitchen clip fades across the ingredient reset. The desk's
25.5-second collaboration clip loops continuously, with the orthographic camera
framed to match its poster. Both scenes suspend playback offscreen and in
background tabs. Reduced motion starts them paused. Each can be played
independently. The poster remains visible before hydration, on asset failure or
after WebGL context loss.

```sh
npm ci
npm run home:three:build
npm run about:three:build  # when shared scene-resources.js changes
cargo build --locked --bin noms --target wasm32-unknown-unknown --no-default-features --features web
python scripts/tests/test-home-kitchen-browser.py --url http://127.0.0.1:8765/ --output /tmp/home-scene-checks
```

Commit the generated home bundle after editing the renderer. Rebuild both bundles
after changing the shared About helpers. The browser checks exercise both scenes,
responsive framing, reduced motion, independent playback, cleanup and fallbacks.
Software WebGL verifies behavior, not phone GPU performance. The kitchen source
and build instructions are in `assets/models/penguini-cookie-kitchen/README.md`.
The editable desk scene is `design/landing/recipe-desk/recipe-desk.blend`. Pack
its web export with `node tools/home/three/pack.mjs INPUT.glb assets/landing/recipe-desk/recipe-desk.glb`,
which verifies the decoded geometry and animation values. Three and Meshopt
licenses are in `tools/about/three/`.
