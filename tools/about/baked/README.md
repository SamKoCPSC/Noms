# Static About-tour bakes

`models.json` maps all twelve active models to preserved inputs. Eight additional
scenes use their original Blender masters. The first kitchen keeps its reviewed
bake. The home office, bedroom and garden have only production GLBs in this
repository: their geometry, vertex colors, UVs, prints and existing Penguini are
recovered by `reconstruct_export.py`, with a new softbox lighting rig. These three
are **not** recovered master shaders or exact reproductions of their original
Cycles lighting. Their provenance is explicit in each lighting JSON.

For native masters, `bake_scene.py` reads the assembled scene without saving over
it, appends the current Penguini placements, preserves shader coordinate systems,
and bakes diffuse direct/indirect light plus emission. Derivative-only surface
repairs correct inward primitives and orient flat painted relief toward the source
camera. Signed-volume repairs only apply to closed shells after welding seams in
a temporary probe; open arches and folded foliage keep their authored winding.
Repairs are listed in the lighting metadata. The two 2048px atlases use
128 Cycles samples and the source view transform/look/exposure. Review-only
renders use up to 32 samples; this does not lower the lighting atlas samples. Glass and metal
remain PBR surfaces with a static reflection environment in Three.js. Original
masters, source hashes and the legacy fallback derivatives remain intact.

From the repository root (Blender 4.3.2, Python, Pillow):

```sh
python tools/about/baked/run_bakes.py corporate-office library supermarket \
  brick-kitchen bakery farmers-market social-kitchen riverside-campsite \
  office bedroom garden --output /tmp/about-bakes --workers 2 --threads 2
python scripts/tests/test-baked-tour.py
```

Each job gets a separate process/output directory. Output is checked for empty
atlases, valid metadata and file size before atomic publication to `assets/about`. Meshopt keeps every triangle,
with 16-bit positions/UVs and 12-bit normals. RGB8 display PNGs preserve every
texel while removing unused alpha and 16-bit storage. Run `npm ci` first.
Review renders are saved under `assets/models/about-baked-tour/previews`.
Intermediate linear EXRs and the full-resolution PNGs remain in the output
folder. To republish an already finished bake, use `--publish-only`. To refresh
only the source-camera reference, append `--reference-only` to a direct Blender
invocation of `bake_scene.py` (after the sample count).

For a material-export-only correction to an unchanged recovered GLB, append
`--reuse-lighting` after the sample count. This rebuilds its deterministic UVs
and reuses the existing lighting images; it checks the source hash first. Keep
the previous packed export and run `node tools/about/baked/verify_material_reexport.mjs BEFORE.glb AFTER.glb`
after packing. It requires byte-identical opaque geometry, UVs and textures, and
checks that live glass/metal retain their vertex colors. Do not use this shortcut
after changing geometry, UV generation, source shaders or lighting.

The first kitchen's original bake script remains in
`assets/models/terracotta-recipe-kitchen/source`; it includes two source-specific
face-winding repairs. Do not run the generic script for that archived bake.

The browser retains the source geometry and separate live glass/metal. This is
more faithful than the legacy vertex-color shader, but real-time glass, metal
and reflections still approximate a Cycles render. Native procedural shading
and stationary shadows are baked, so assembly animations are intentionally
removed; Rust's camera flight and text transitions continue unchanged.
