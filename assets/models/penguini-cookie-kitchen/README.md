# Penguini’s cookie kitchen

The Noms ivory, terracotta, sage and charcoal scene, with the right wall, open
left side, no tea towel, and cookie tray resting on the countertop. The Blender
4.3.2 master keeps all editable geometry and the complete 45.2-second sequence: welcome, flour,
happy mixing, recipe reading, chocolate chips, happy mixing, return to rest.

`assets/landing/kitchen/penguini-kitchen.glb` is the browser derivative. Stationary
surfaces use two embedded 2048px, 128-sample display-baked atlases. Animated
objects remain live; the book, flippers, facial expressions and ingredient pours
retain their keyframes. Steam stays translucent. The character uses the approved
lighter preview geometry; the full-detail master is unchanged. Lighting metadata
records its SHA-256 and bake provenance. The renderer shares About’s lighting and
resource-disposal helpers. No real-time shadow maps are required.

Kitchen colors come from the light-theme tokens in `assets/main.css`: terracotta
cabinetry, warm ivory walls and countertops, pale sage backsplash, green ceramic
accents and charcoal stove details. Penguini, the Noms book, food and natural
wood/metal finishes retain their existing colors. The palette stays the same in
light and dark mode. The lighting manifest records the source tokens and colors.

To rebuild, copy the master and the scripts in `source/` into an external
working directory (bakes generate large intermediates). Run there:

```sh
# Optional: refresh material colors if the app palette has changed.
blender -b -t 4 --python apply_noms_palette.py -- penguini-cookie-kitchen-baking.blend /path/to/Noms/assets/main.css
blender -b -t 6 --python bake_static.py
blender -b -t 4 --python build_preview_lod.py
```

Then from the repo root:

```sh
node tools/home/three/pack.mjs WORK/preview-lite.glb assets/landing/kitchen/penguini-kitchen.glb
```

Copy `WORK/lighting-bake.json` to `assets/landing/kitchen/lighting.json` after a
rebake. The source script records Three-coordinate softbox positions and Blender
rotations for the shared reflection environment. Packing preserves all vertex
and animation values; Meshopt may cyclically rotate triangle indices without
changing winding. The pack command verifies this after decoding the export.

The GLB contains a single clip. Infinite playback, a brief ingredient-reset fade,
pause/resume, reduced motion and offscreen suspension belong to the web renderer.
The poster is an intentionally static fallback, not a validation screenshot.
