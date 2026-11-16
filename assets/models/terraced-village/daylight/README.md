# Shared daylight throughout the village

All twelve locations and the outdoor world use one warm sun and a brighter, cool
sky fill. The sky strength is 0.95 (the kitchen trial used 0.5), with exposure +0.1
instead of −0.18. These settings keep shaded interiors readable while retaining
contact shadows. The matte surfaces, rounded world geometry, existing room
contents, placements and walking route are preserved.

The editable [native world](models/terraced-village-daylight.blend) includes all
original room shaders, packed images, one sun and the walking camera. It contains
24,226 source objects in twelve collection instances, plus the outdoor landscape.
The original world and individual masters are retained separately, unchanged.

The browser assets are derivatives of this native scene:

- Each room has two 2048 × 2048 display-baked atlases, sampled at 64 rays per texel.
  Nearby rooms and landscape within 45 world units provide shadow and bounce
  context. Every room uses the same sun, sky and display settings.
- Four 2048 × 2048 ground tiles cover the village and receive shadows from the
  complete native world, at 32 samples per texel.
- Rounded paths, steps, houses, signs and other landscape surfaces use baked
  vertex colors at 64 samples. This avoids dark seams from tiny UV islands.
- Opaque lighting is already display transformed with Blender AgX and is not
  lit or tone mapped again in Three.js. Glass and reflective metal retain
  view-dependent rendering with an outdoor reflection environment.

These are static lighting bakes; the preview does not use real-time shadow maps.
Glass reflections are still a browser approximation, not a pixel-identical
Cycles render. All locations remain resident for overview review, so this is not
a streaming or mobile-memory optimization pass. Some exports, including the
bedroom, home office and garden, retain more native geometry than the older
browser derivatives; triangle counts are recorded in the asset receipts.

## Rebuild

Run from the repository root with Blender 4.3.2, Python with Pillow, and the
repository's Node dependencies. Two room workers with two threads each fit the
four-core bake machine; run the complete-world bakes afterward to bound memory.

```sh
python -m pip install numpy shapely==2.2.0
python tools/about/village/stone-paths.py
python tools/about/village/vegetation.py
node tools/about/village/daylight/anchors.mjs
blender -b -t 2 --disable-autoexec --python-exit-code 1 --python tools/about/village/daylight/build-world.py -- assets/models/terraced-village/daylight/models/terraced-village-daylight.blend
python tools/about/village/daylight/run.py /tmp/village-daylight --workers 2 --threads 2
blender -b -t 4 --disable-autoexec --python-exit-code 1 --python tools/about/village/daylight/bake-ground.py -- /tmp/village-daylight/world
blender -b -t 4 --disable-autoexec --python-exit-code 1 --python tools/about/village/daylight/bake-landscape.py -- /tmp/village-daylight/world
node tools/about/village/daylight/pack-world.mjs /tmp/village-daylight/world
node tools/about/village/daylight/check-assets.mjs
blender -b -t 2 --disable-autoexec --python-exit-code 1 --python tools/about/village/daylight/check-native.py
node tools/about/village/build-preview.mjs /tmp/noms-village-daylight
node tools/about/village/build-mobile-preview.mjs /tmp/noms-village-daylight /tmp/noms-village-daylight-mobile
```

Room bakes resume only when settings and packed-asset hashes match. The renderer
uses the original bounds to preserve the approved camera framing despite joining
meshes for export. Geometry compression removes no triangles. The single-file
preview uses WebP quality 76 at the original texture dimensions and verifies
geometry bytes against its input GLBs. This keeps the ZIP below GitHub’s 100 MiB
file limit; the repository GLBs retain their lossless PNG textures.

[Lighting settings](lighting.json), per-asset bake receipts and verification
reports are stored alongside the assets in `source/`.

## Review evidence

- [All twelve arrival views](previews/all-locations.png)
- [Native world verification](source/native-verification.json)
- [Thirteen exported assets](source/asset-verification.json)
- [Desktop and portrait browser checks](source/browser-verification.json)
- [Single-file browser checks with external requests blocked](source/offline-browser-verification.json)
- [Texture conversion and geometry receipts](source/mobile-package-receipts.json)

Both preview formats passed all twelve stops, playback, responsive aspect and
2,211 walking-route samples. Camera stops match the previous layout exactly.
These checks ran in desktop Chromium with software rendering; direct Android
file launching and phone performance have not been measured.

## Connected stone paths (revision 8)

The old independently extruded path strips and entrance forecourts are replaced
by one unioned footprint. Terrain-following, softly beveled limestone pieces use
small variations of warm gray. A continuous grout bed joins the entire network,
including all twelve entrance forecourts. The bed extends into the terrain; it
has no floating open ends. Existing steps and room interiors remain in place.

The builder partitions the complete footprint into non-overlapping stone cells
before adding joints and bevels. One continuous height function follows the ground
and blends into the level forecourts, so crossing branches cannot disagree about
height. It verifies footprint connectivity, partition coverage, overlap area,
terrain clearance, every walking route and all twelve entrance connections.

[Path geometry checks](source/stone-path-verification.json) and the compressed
geometry source are retained. `common.environment()` replaces the old paths from
the baseline environment when rebuilding the current native world. The baseline
world and its geometry snapshot remain unchanged as source inputs. Landscape and
ground shadows are rebaked with the existing bright lighting; room GLBs are retained.

## Textured meadow and folded foliage (revision 9)

The outdoor ground now has restrained procedural color variation, 13,177 grass
clumps with tapered folded blades, and 200 patches of ivory daisies, golden
buttercups and lavender spikes. The 2,476 flowers have shaped petals, stems and
basal leaves. Petal tops face upward for the daylight bake; the generator verifies
their normals and checks every groundcover vertex against the paving. Vertex albedo gradients brighten blade tips without shiny materials.
These are static meshes with shared baked daylight; no animated grass or extra
real-time shadow maps are introduced.

Broadleaf crowns and shrubs have lobed silhouettes and layered creases. Evergreen
tiers have longitudinal folds. The 999 replacement foliage components fit inside
their original bounding boxes. New groundcover avoids rooms, the stone network,
river banks, cottage foundations, porches, rocks, steps and planting beds. The
approved cameras and all twelve room GLBs remain unchanged.

`vegetation.py` writes a deterministic compressed geometry layer and placement
checks. `common.environment()` applies its baseline-index replacements before
applying the stone-path layer. It verifies the original environment snapshot hash;
that baseline is never overwritten. The current native world, ground atlases and
vertex-lit landscape are rebuilt from both layers. Source vertex colors are baked
into `WorldLighting` and removed from the export to avoid redundant color channels.
The packed unlit world omits unused normal/tangent attributes; its native source
retains the shading normals. Positions, triangles and baked colors are preserved.

- [Vegetation geometry and placement checks](source/vegetation-verification.json)
- [Meadow detail at walking height](previews/meadow-detail.png)

The source layer adds 568,432 triangles before replacing the old vegetation.
Final exported counts and download sizes are recorded in the asset and package
receipts. This remains a full-world review prototype, not a phone performance
benchmark.
