# About tour kitchen

Web derivative of Sam's approved **revision 11** Blender kitchen. The editable master and source renders remain on the [model review page](https://chatgpt.com/space/page_a00991c491f4819185d71cf3ade6f712); this directory contains the production derivative, not the native master.

- `sams-kitchen.glb`: 54,988 triangles, one mesh/material/draw call, baked vertex colors and soft contact shading. Standard `KHR_mesh_quantization` reduces the file to 1,588,000 bytes. Uniform position quantization is approximately 0.11 mm per step in the source units.
- `sams-kitchen.webp`: 960px still image used in reading mode, reduced motion and graphics fallbacks.
- `kitchen-budget.json`: generated budget/provenance record.

## Rebuild

Requires Blender 4.3.2 and Python 3. The approved master is read without modifying or saving it. Export to a temporary directory before copying completed outputs here so the app watcher cannot read a partial asset.

```sh
blender -b /path/to/revision-11/sams-kitchen.blend --threads 5 \
  --python tools/about/export_kitchen.py -- /tmp/noms-kitchen-web
python tools/about/pack_kitchen.py /tmp/noms-kitchen-web
cp /tmp/noms-kitchen-web/{sams-kitchen.glb,kitchen-budget.json} assets/about/
```

The exporter evaluates rounded edges, simplifies silhouettes, joins the diorama and bakes subtle contact shading into the material palette at mesh corners. The packer removes unused attributes and quantizes positions, normals and colors. No image textures or UV atlas are required by the kitchen. The app reads this deliberately small GLB contract in Rust; arbitrary third-party GLBs are not supported by this loader.

## Runtime

`src/pages/about_tour` ports the approved first-person Rust/WebGL prototype into the Dioxus About route. Semantic HTML contains all four complete chapters before enhancement. The camera and floating panels share the same projection, scoped beneath the existing app navbar. The kitchen is fetched from the local Manganis asset URL near the creator stop; other routes and the initial tour stop do not fetch it.

Mobile starts with ambient drift paused. Camera travel still follows native scrolling. Pause/read controls, reduced-motion changes, WebGL/context-loss fallbacks and route unmounting stop rendering or dispose resources. Restoring graphics rebuilds the scene and reloads the kitchen if needed. Reading mode uses the lazy still image.

## Checks

```sh
cargo fmt --all -- --check
cargo check --locked --target wasm32-unknown-unknown --no-default-features --features web
cargo test --locked --no-default-features --features server pages::about
cargo test --locked --no-default-features --features server pages::marketing_contract_tests
```

The GLB contract test checks the delivered mesh/file budget, finite geometry and preservation of the color palette. Browser verification should cover all four stops, forward/backward scrolling, creator panel scrolling, deferred loading, mobile idle rendering, route exit/re-entry, reduced motion, graphics and image failure, context recovery, and desktop/mobile screenshots. Do not commit browser validation screenshots to the application or PR.
