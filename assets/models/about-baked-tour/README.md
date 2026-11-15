# About-tour baked lighting review

The active tour uses twelve static Three.js scenes, sharing Rust camera control.
`previews/*-reference.webp` shows the source scene used for each new bake.
Original masters are never overwritten. Production exports and lighting metadata
live in `assets/about`, alongside the preserved fallback derivatives.

The office, bedroom and garden reference renders use recovered exported geometry,
vertex colors and prints with a new softbox rig. They are not original master
renders; the source Blender files were not available in the repository. The
other references use the archived source materials, lights and color management,
with the approved Penguini companions added non-destructively.

See [reproduction and provenance](../../../tools/about/baked/README.md) and
[renderer behavior](../../../tools/about/three/README.md).
