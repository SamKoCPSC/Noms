# Landing-page 3D scenes

`HomeScene` owns each model's poster, pause control, reduced-motion preference
and route cleanup. All scenes share one locally bundled Three.js renderer.
The kitchen loads in the hero. The recipe desk, Pandacotta’s banquet, and Owlfredo’s soup island load
when their respective sections approach the viewport.

The kitchen plays the full recipe and fades as the tray and bowl reset. The desk
loops its 25.5-second collaboration clip. In “Discover new recipes,” Pandacotta
uncovers and tastes three dishes in a 33-second loop. Its poster shows all three
dishes, and playback begins with the domes closed.
Under “Make cooking your favorites convenient,” Owlfredo seasons and tastes soup,
then hops twice before his 28-second loop restarts.

Each scene has its own pause control, suspends playback offscreen and in
background tabs, and starts paused for reduced motion. Its poster is visible
before hydration, on asset failure, or after WebGL context loss.

```sh
npm ci
npm run home:three:build
npm run about:three:build  # when shared scene-resources.js changes
cargo build --locked --bin noms --target wasm32-unknown-unknown --no-default-features --features web
python scripts/tests/test-home-kitchen-browser.py --url http://127.0.0.1:8765/ --output /tmp/home-scene-checks
```

Commit the generated home bundle after editing the renderer. Rebuild both bundles
after changing the shared About helpers. The browser checks exercise the scenes,
responsive framing, reduced motion, independent playback, cleanup and fallbacks.
Software WebGL verifies behavior, not phone GPU performance. The kitchen source
and build instructions are in `assets/models/penguini-cookie-kitchen/README.md`.
The editable banquet scene is `design/landing/pandacotta-banquet/pandacotta-banquet.blend`,
with its runtime GLB and poster in `assets/landing/pandacotta-banquet/`.
The editable soup island is `design/landing/owlfredo-kitchen-island/owlfredo-kitchen-island.blend`,
with its runtime GLB and poster in `assets/landing/owlfredo-kitchen-island/`.
The editable desk scene is `design/landing/recipe-desk/recipe-desk.blend`. Pack
its web export with `node tools/home/three/pack.mjs INPUT.glb assets/landing/recipe-desk/recipe-desk.glb`,
which verifies the decoded geometry and animation values. Three and Meshopt
licenses are in `tools/about/three/`.
