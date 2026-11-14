# Riverside Cabin Campsite

Intended About-tour section: **Respecting your privacy.**

Revision 4 is a new reconstruction from the approved design discussion. Earlier uncommitted workspace artifacts were lost; this is not a byte-identical recovery. The sources, native master, GLB, screenshots and download bundle are now versioned together in the repository.

Corner composition with a two-storey cedar cabin, river, no fence or tent, a wooden slatted porch lounge, five simple tiered evergreen trees with shallow geometric folds, hot dogs, baked beans, corn, smores, marshmallows, and enamel mugs.

## Files

- [Download complete version 4 bundle](downloads/riverside-cabin-campsite-v4.zip) (use GitHub’s **Download raw file** button).
- [Editable Blender 4.3 master](models/riverside-cabin-campsite.blend).
- [Standard GLB export](models/riverside-cabin-campsite.glb).
- [Assembly roots and bounds](models/riverside-cabin-campsite-assemblies.json).
- [Validation report](models/validation.json).
- [Rebuild source](source/build_scene.py).

The ZIP contains the model, source, previews, documentation, and SHA-256 checksums. Source is also embedded in the Blender master at build time. `downloads/` is excluded from the ZIP to avoid recursive archives.

## Rebuild and verify

From this directory:

```sh
blender -b --python-exit-code 1 --python source/build_scene.py -- --skip-render
blender -b --python-exit-code 1 --python source/render_review.py
blender -b --python-exit-code 1 --python source/validate_model.py
python source/package_model.py
```

`--proof` on the build command produces a smaller review render. Validation checks native scene intent, finite transforms, GLB structure and buffer bounds, and a fresh Blender import. The kitchen additionally checks food vertex colors and board/butter placement; the campsite checks its two storeys, wooden lounge, folded trees and absence of tent/fence.

These are static review assets, not production About-tour derivatives. Named assembly roots carry suggested entrance offsets. Lighting and shadows are dynamic in Blender; the web viewer supplies its own lights. Geometry, PBR materials and food vertex colors export to GLB; Blender procedural micro-bump textures do not. App integration and loader optimization are separate work.

## Actual Blender renders

![Full model](previews/riverside-cabin-campsite.png)

![Campsite detail](previews/campsite-details.png)
