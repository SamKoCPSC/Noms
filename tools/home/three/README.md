# Landing-page 3D scenes

`HomeScene` owns each model's poster, pause control, reduced-motion preference
and route cleanup. All scenes share one locally bundled Three.js renderer.
The kitchen loads in the hero. The recipe desk, Pandacotta’s banquet, the plating
scene, Owlfredo’s soup island, Terrimisu’s cookie study, and the bakery stand load
as their sections approach the viewport.

The kitchen plays the full recipe and fades as the tray and bowl reset. The desk
loops its 25.5-second collaboration clip. In “Discover new recipes,” Pandacotta
uncovers and tastes three dishes in a 33-second loop. Its poster shows all three
dishes, and playback begins with the domes closed. In “Create your signature
recipe,” Terrimisu plates ravioli for Owlfredo in a 24-second loop. With reduced
motion enabled, this scene shows its poster until the visitor presses Play.
Under “Make cooking your favorites convenient,” Owlfredo seasons and tastes soup,
then hops twice before his 28-second loop restarts. In “Study how your recipes
change,” Terrimisu tastes two cookie batches in a 24-second loop. Visitors who
prefer reduced motion can start the cookie tasting from its static poster.
In “Share your best creations,” Penguini tastes a berry swirl and receives its recipe
from Pandacotta. The 27-second bakery animation waits for Play when reduced
motion is enabled.

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
The cookie study source is `design/landing/terrimisu-cookie-study/terrimisu-cookie-study.blend`.
Export the scene animation as a single GLB clip with its shape keys, excluding the
studio floor, lights, and camera. Pack the web model with
`npx --yes --package gltfpack@0.25.0 gltfpack -i INPUT.glb -o assets/landing/terrimisu-cookie-study/terrimisu-cookie-study.glb -cc -si 0.35 -se 0.003 -af 0`.
The editable bakery stand is `design/landing/pandacotta-market-stand/pandacotta-market-stand.blend`,
with its Meshopt-packed GLB and handoff poster in `assets/landing/pandacotta-market-stand/`.
The editable desk scene is `design/landing/recipe-desk/recipe-desk.blend`. Pack
its web export with `node tools/home/three/pack.mjs INPUT.glb assets/landing/recipe-desk/recipe-desk.glb`,
which verifies the decoded geometry and animation values. Three and Meshopt
licenses are in `tools/about/three/`.

The editable plating scene and full-resolution GLB are in
`assets/brand/signature-recipe/`. From the repository root, rebuild the landing
GLB from the full export with the same mesh simplification and animation settings:

```sh
npx --yes --package gltfpack@0.25.0 gltfpack -i assets/brand/signature-recipe/terrimisu-and-owlfredo.glb -o assets/landing/signature-recipe/terrimisu-and-owlfredo.glb -cc -si 0.23 -se 0.003 -af 0
```
