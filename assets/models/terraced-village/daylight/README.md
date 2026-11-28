# Shared daylight throughout the village

All twelve locations and the outdoor world use one warm sun and a brighter, cool
sky fill. The sky strength is 0.95 (the kitchen trial used 0.5), with exposure +0.1
instead of −0.18. These settings keep shaded interiors readable while retaining
contact shadows. The matte surfaces, rounded world geometry, existing room
contents, placements and walking route are preserved.

The editable [native world](models/terraced-village-daylight.blend) includes all
room shaders with the clear-glass corrections, packed images, one sun and the walking camera. It contains
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
  Cottage walls have separate normals on the flat panels and window reveals.
  A half-metre grid gives the bake enough samples to resolve shadows under the
  eaves and around the frames.
  The four wayside cottages share their plot and doorstep coordinates with the
  limestone and meadow generators. Each approach joins the paving before the
  stones are laid. Roof and trim lighting samples sit just inside each face so
  shadows at the edges of a course stay local. [Cottage placements and roof styles](../source/wayside-cottages.json)
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
repository's Node dependencies. With dense landscape context, use one room worker and four threads on a
16 GiB machine; run complete-world bakes afterward to bound memory. The runner
also supports multiple workers when more memory is available.

```sh
python -m pip install numpy shapely==2.2.0 zopfli
python tools/about/village/stone-paths.py
python tools/about/village/vegetation.py
node tools/about/village/daylight/anchors.mjs
python tools/about/village/daylight/run.py /tmp/village-daylight --workers 1 --threads 4
blender -b -t 2 --disable-autoexec --python-exit-code 1 --python tools/about/village/daylight/build-world.py -- assets/models/terraced-village/daylight/models/terraced-village-daylight.blend --glass-source /tmp/village-daylight/clear-rooms.blend
blender -b -t 4 --disable-autoexec --python-exit-code 1 --python tools/about/village/daylight/bake-ground.py -- /tmp/village-daylight/world
blender -b -t 4 --disable-autoexec --python-exit-code 1 --python tools/about/village/daylight/bake-landscape.py -- /tmp/village-daylight/world
node tools/about/village/daylight/pack-world.mjs /tmp/village-daylight/world
node tools/about/village/daylight/check-assets.mjs
blender -b -t 2 --disable-autoexec --python-exit-code 1 --python tools/about/village/daylight/check-native.py
node tools/about/village/build-preview.mjs /tmp/noms-village-daylight
node tools/about/village/build-mobile-preview.mjs /tmp/noms-village-daylight /tmp/noms-village-daylight-mobile
python tools/about/village/package-mobile-preview.py /tmp/noms-village-daylight /tmp/noms-village-daylight-mobile /tmp/noms-village-mobile.zip
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

## Denser grass (revision 10)

The meadow now has 28,080 tufts and 84,240 blades, up from 13,177 tufts and
52,683 blades. Closer, jittered spacing and fuller coverage in the open grass
reduce the bare patches. Each tuft has three slightly broader blades. Their
buried underside caps are omitted, limiting the increase to 41,988 triangles
while retaining all visible folded faces. The existing clearance around paths,
entrances and flower patches is preserved.

Flowers, trees, shrubs and other non-grass source meshes are byte-for-byte
identical to revision 9. The native world and both outdoor lighting passes are
rebuilt; room assets and camera poses stay unchanged. The
[density comparison](source/grass-density-verification.json) records the counts
and preservation check.

The downloadable ZIP uses Zopfli to optimize standard Deflate compression without
changing the embedded HTML or model data. Ordinary ZIP readers can extract it;
no additional browser decoder is required. The packager verifies file checksums
and extracted bytes before publishing the archive.


## Village names and sign mounts (revision 11)

The direction boards now use the same playful place names as the tour menu.
Destination lettering reserves space for the arrows on both faces. Street posts
have a slim oval cross-section contained within the sign backing and terminate
below the badge edge. Road noticeboard and welcome-board supports sit behind
their backing boards, with positive clearance.

Signs are stored in `source/signage.json.gz` as a replacement layer over the
immutable world snapshot. Grass, flowers, paths, houses, room models and camera
poses are preserved. `signage-verification.json` records names, sign positions,
mount clearances and the geometry hash. Native and bake receipts verify that
same hash; browser checks verify both face clearances and user-facing names.

To regenerate this layer after editing `neighborhood.js` or the layout titles,
build and serve a preview first, then run:

```sh
python tools/about/village/export-signage.py http://127.0.0.1:8766
```

Then rebuild the native world and the two outdoor lighting passes, pack the
world and rebuild the normal/mobile previews using the commands above.

## Clear glass and window openings (revision 12)

`daylight/glass.py` identifies opaque window substitutes by explicit material and
room names. It converts those panes and authored transmitting glass to Principled
transmission 1, roughness 0, IOR 1.45, metallic 0 and alpha 1. Clear panes are neutral;
the library's colored art glass retains its tint. Shader replacements remove frost
normal/roughness maps and fake emission. Screens, smoked appliance panels, liquids,
and glazed food are not treated as architectural glass.

The first kitchen, garden house and cabin receive actual openings through their
wall/door backing. The solid cabin and eight background cottages are hollowed;
cottage window casings become frames. The bedroom's painted sky and hills are
hidden in the derivative so its windows show the village. Trim and mullions stay.
The archived baseline and individual Blender masters are never overwritten.

Thin clear glass passes shadow rays during the diffuse bake, avoiding opaque black
shadows from unsupported refractive caustics. Camera and reflection rays still see
the physical glass. Display-baked opaque surfaces remain unlit in Three.js; clear
glass stays live with full-resolution transmission captures and no second tone map
of the baked view behind it. `clear-glass.js` uses a level-zero transmission lookup
for explicitly clear glass, avoiding Three.js's minimum-roughness/bicubic blur while
retaining Fresnel, IOR and reflected-highlight anti-aliasing. Clear panes also
avoid applying distance fog a second time to the already fogged transmission
image, reducing excess haze in distant views. A pixel regression
checks a sharp stripe pattern through a physical pane: the standard shader softens
38 sampled pixels, while the clear shader and unobstructed view soften none. Run
`python tools/about/village/check-clear-glass.py /tmp/clear-glass-check` to repeat it. This does not introduce real-time shadow maps.

The room runner caches the corrected native source once, checks its policy and
original-file hashes, and invalidates old bakes when the glass policy changes.
`--only bakery` supports a focused visual check before rebuilding all locations.
The full native world retains the corrected materials and geometry, and
`source/glass-native.json` records every material, opening and hidden backdrop.
Asset verification checks exported transmission, roughness, tint, absence of frost
textures, and the current policy hash alongside normal geometry/lighting checks.

## Outdoor tour trees

The browser world grows cherry blossoms, firs, autumn maples and green oaks at
the 116 planting locations in the landscape snapshot. Both outdoor Cycles passes
replace the native scene's original trees in memory and import the tour's trunks,
branches and leaf cutouts at their planted positions. The ground textures and
the vertex lighting on grass, paths and outdoor props receive their shadows. The
leaf atlas uses the same alpha cutoff of 0.4 in the bake and in the browser.
The bake receipts record the tree geometry and material hash for verification.
[Compare outdoor tree shadows before and after the bake.](previews/village-tree-shadows.jpg)

The four species share an embedded cutout atlas. Each has four seeded canopy
shapes that the renderer instances at different sizes and orientations. Explore
uses solid trunk and branch geometry for collision and lets players pass through
leaf sprays. The gallery shows the trees at walking height. [Village tree gallery](previews/village-trees.jpg)

After changing the tree shapes, placements or leaf atlas, rebuild the outdoor
lighting and the shipped world from the repository root. The atlas painting step
is only needed after editing the sprig artwork.

```sh
python tools/about/village/trees/paint-sprigs.py
blender -b -t 4 --disable-autoexec --python-exit-code 1 --python tools/about/village/daylight/bake-ground.py -- /tmp/village-daylight/world
blender -b -t 4 --disable-autoexec --python-exit-code 1 --python tools/about/village/daylight/bake-landscape.py -- /tmp/village-daylight/world
node tools/about/village/daylight/pack-world.mjs /tmp/village-daylight/world
node tools/about/village/daylight/check-assets.mjs
node tools/about/village/build-app-assets.mjs
```

## Sculpted landscape rocks

The two outdoor Blender bakes run `tools/about/village/daylight/rocks.py` on the
native village. It uses the original 81 boulder locations and footprints to
place fractured limestone blocks, low slabs, paired pinnacles and loose chips.
Sloping ledges pinch out around the sides, and clipped ridges break up the top
silhouettes. The daylight bake lights the stone faces and casts shadows onto
the surrounding terrain. Explore collision comes from the exported geometry.
[Studio view of four village formations](previews/village-rock-study.jpg) ·
[Before and after in the tour](previews/village-rocks.jpg)

To rebuild after changing a rock shape or color, run both outdoor bakes and the
packing and verification commands in the tree section above. Both bake receipts
record a hash of the finished stone geometry and its material assignments.
