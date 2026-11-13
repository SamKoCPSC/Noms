# UBC-inspired reading hall — revision 4

Remodeled from the two user-supplied photographs of the UBC reading hall. This replaces the first reconstruction's dominant bookcase wall with the reference's double-height architecture, portrait gallery and window-facing seats. It is a stylized interpretation, not a measured architectural replica.

Features:

- Honey-coloured timber wall panels and a grey academic portrait wall.
- Tall divided windows filled with simple opaque matte blue panels; no exterior trees or garden.
- Eleven suspended coloured-glass ribbons, with faceted colour and etched patterns.
- Eleven original stylized academic portraits in varied frames. These do not claim to depict the actual people in the reference photos.
- A 24-tread spiral staircase with a measured 360-degree turn, continuous stringers and connected handrails. It leads to an open L-shaped second-floor section; there is no doorway.
- Two long reading tables, twelve wooden spindle chairs and six domed brass lamps.
- Four upholstered chairs facing the windows, each with a small reading table.
- Four small laptops with varied code screens, five open computer-science textbooks with curved pages, printed lines and bookmarks, closed books, notes and a return trolley. The CS collection now occupies low shelving beneath the portraits.

The matte material style and live shadows match the other Noms dioramas. The suspended art glass remains translucent; the window panels are opaque. There are no baked object shadows or external texture/font dependencies.

## Files

- `models/university-library.blend`: editable Blender master, geometry, text, lighting and camera.
- `models/university-library.glb`: evaluated geometry/materials for review; not a production About-tour derivative.
- `models/university-library-assemblies.json`: thirteen independent assembly groups. Suggested offsets use Blender coordinates; animations are not baked.
- `source/`: deterministic rebuild scripts, adapted from the Noms community-scene primitives.
- `previews/`: actual Blender renders, including the full room and a closer view.
- `native-validation.json` / `import-validation.json`: native geometry and GLB round-trip checks.
- `source-checksums.json`: integrity hashes for the editable master, export and scripts.

## Rebuild

Run from this directory with Blender 4.3 or compatible:

```sh
blender -b -t 4 --python source/build_library.py
blender -b -t 4 --python source/render_review.py
blender -b -t 4 --python source/validate_export.py
```

Append `-- --proof` to the build command for a faster draft render. Final review renders use 64 Cycles samples without requiring OpenImageDenoise.

Earlier revisions were preserved separately before remodeling. This approved revision is integrated into the About tour’s origin-story card. The optimized derivative and its geometry audit live in `assets/about/`; the master remains unchanged. See [web reproduction instructions](../../about/README.md#library-and-supermarket).
