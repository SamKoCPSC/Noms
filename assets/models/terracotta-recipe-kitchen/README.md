# Terracotta recipe kitchen — original master

The original revision 13 Blender master for the first About-tour kitchen, restored
from the user attachment on 2026-11-15. The file is preserved byte-for-byte; see
[source-receipt.json](source-receipt.json) for its SHA-256 and provenance.

[Download the Blender master](models/sams-kitchen-recipe-manager.blend) ·
[Original model review](https://chatgpt.com/space/page_60c2cf7e836c8191a4c07002a7fdfc0c)

Verified with Blender 4.3.2: the file opens at the assembled frame 160, contains
18 assembly controls, the original three area lights and approved camera, and
six packed reference/material images. The laptop screen, cookbook artwork and
croissant textures do not depend on the original machine's absolute file paths.
Color management is AgX / Medium High Contrast with exposure -0.18.

The existing optimized app derivative remains in `assets/about/`. Preserve this
master when producing new web exports or baked lighting; work in separate files.

## Static Three.js derivative

[Compare the Blender master and browser render](previews/comparison.jpg) ·
[Browser render](previews/three-kitchen.webp)

Both comparison images use the source camera and the current Penguini companion.

The derivative in `assets/about/kitchen-baked.glb` comes from this master at frame
160. It includes the current Penguini companion, two embedded 2048px lighting
atlases, and separate glass/metal materials. The original `.blend` is never
saved over. Its geometry, materials and lighting remain available for revisions.

From the repository root, with Blender 4.3:

```sh
blender -b assets/models/terracotta-recipe-kitchen/models/sams-kitchen-recipe-manager.blend \
  --threads 4 \
  --python assets/models/terracotta-recipe-kitchen/source/bake_three_kitchen.py \
  -- /tmp/kitchen-three-bake
node tools/about/baked/pack.mjs /tmp/kitchen-three-bake/kitchen-baked.glb \
  assets/about/kitchen-baked.glb /tmp/kitchen-three-bake/lighting.json
cp /tmp/kitchen-three-bake/lighting.json assets/about/kitchen-lighting.json
python scripts/tests/test-baked-kitchen.py
```

The script renders a master reference, bakes 128-sample Cycles diffuse lighting
and emission with all original lights present, and saves linear EXRs as well as
display PNGs. The PNGs use the master’s AgX / Medium High Contrast / -0.18
exposure settings. Bake buffers must use **Linear Rec.709**, not Non-Color,
so Blender applies that view transform when saving the display images.

The web export reverses the inward face winding of the hand-built sourdough
loaf and pot lid before baking. Blender’s two-sided rendering hides that issue
in the master, but uncorrected bake rays would sample their dark interiors.
For a detail-only iteration with unchanged architecture and lighting,
`--reuse-architecture` can reuse an existing output directory’s architecture
atlas and master reference; use the full command for a fresh build.

The export finalizer marks those textures `KHR_materials_unlit`; the browser
must not light or tone-map them again. Glass and metal remain view dependent
and use a static reflection environment. The reference camera and source light
settings are recorded in `assets/about/kitchen-lighting.json`.
The full-geometry export has 447,855 triangles. The shared tour now packages it
with Meshopt and RGB8 display PNGs at approximately 10.7 MB, preserving every
triangle and the 2048px atlas resolution. Run `tools/about/baked/pack.mjs` after
a fresh bake; keep the uncompressed output and linear EXRs for future revisions.

See [renderer notes](../../../tools/about/three/README.md) for build instructions,
fallbacks, browser verification and fidelity limits.
