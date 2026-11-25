# Village About tour (current default)

The About page now renders the reviewed terraced village as one full-viewport
Three.js world. `src/pages/about_tour/village_browser.rs` owns navigation,
scrolling, card fades, the loading curtain, reduced-motion reading mode and
cleanup. `village.js` owns only asset loading and rendering; it has no animation
loop or navigation event handlers. The previous independent-room adapter remains
below as a reference and for pages without the village asset catalog.

Desktop reserves the left side for the text overlay and frames the current room
on the right. Mobile puts the overlay above the room. `village-camera.js` uses
an off-axis projection over the entire canvas, preserving proportions without
cropping the scene behind the text. It evaluates the existing collision-checked
walking-camera ground route, with smoothly interpolated elevation offsets for a
32° downward view at each stop. All room placements stay unchanged. Back/Next page,
Page Up/Down and left/right arrows move between stops in 6400 ms with quintic
ease-in/ease-out (zero starting and ending velocity and acceleration). The
camera turns toward the path over the first 36% of each leg and toward the
destination over the last 36%. Reversing a scroll eases from the current bearing.
Native scrolling over the scene or cards controls the camera directly, including
when paused between sections. Each full leg spans 3.4 viewport heights of scrolling.
Explore village places the eyes 1.65 m above the floor in each room’s original
model scale. On the approach, eye height blends smoothly between the meadow
and the enlarged room over the width of that room. Visitors can cross the paths,
lawns and room footprints inside the timber fence. Its rails follow the terrain around the master layout's
220 × 260 m ground outline. The walking boundary uses those same edges and
leaves 0.3 m between the camera and the inside face of the fence posts.
WASD and the touch stick start and stop immediately, with a top walking speed of
5 m/s. Hold Shift to sprint at 10 m/s, press Space to jump, or hold Left Ctrl
to crouch and walk at 2.5 m/s. Eye height bobs subtly with each grounded step,
settling when movement stops. A jump rises one village unit in 0.40 seconds
and lands 0.37 seconds after its peak.
Use the mouse, a touch drag or the arrow keys to look. Escape releases the mouse
and stops movement. Resume tour eases back to the saved pose over 1.2 seconds.

The opening approach takes 4.8 seconds.
Direct links and browser history retain the story destinations.

The loader waits for all thirteen world-space GLBs and the first rendered frame.
Two asset requests run concurrently. Unlike the old adapter, the whole village
stays resident because other locations remain visible during a walk; it does not
also load the legacy GLBs. The renderer sleeps at rest and disposes geometry,
textures, reflection targets and pending requests on route exit. Unavailable
WebGL, failed assets, lost context, reduced motion and very short viewports use
the still-image reading page. A reading escape remains available while loading.

The app derivatives live in `assets/about/village/`: geometry and clear glass
match the approved preview, with the same WebP quality 76, original-resolution
baked atlases. The package is about 136 MB before HTTP compression, so this still
needs physical-phone memory/frame-rate profiling before making performance
claims. Native masters and full-quality bakes remain in the models archive.

```sh
# Regenerate app GLBs only after changing the reviewed village/bakes:
node tools/about/village/build-app-assets.mjs
# Camera-only changes do not require repacking the GLBs:
node tools/about/village/build-camera.mjs
npm run about:three:build
node --test tools/about/three/village-{camera,walk}.test.mjs
cargo check --bin noms --target wasm32-unknown-unknown --no-default-features --features web
python scripts/tests/test-about-village-browser.py http://127.0.0.1:8765/about /tmp/about-village-check
```

Commit `assets/about-three-village.js` alongside the source. The camera test
checks all twelve stops at desktop and mobile sizes plus every verified walking
sample. The browser test exercises the real Rust About component, the complete
world, framing, button timing, wheel gestures over text, reading mode, context
loss and loading failures. Chromium software rendering verifies behavior, not
hardware performance.

## Previous independent-room adapter

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
