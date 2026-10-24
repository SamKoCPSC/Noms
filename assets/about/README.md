# About tour kitchen

Web derivative of Sam's approved **revision 13** Blender kitchen. The editable master and source renders remain on the [model review page](https://chatgpt.com/space/page_60c2cf7e836c8191a4c07002a7fdfc0c); this directory contains the production derivative, not the native master.

- `sams-kitchen-detailed.glb`: 187,949 triangles, 18 assembly groups and 20 draw calls (two additional print surfaces), baked vertex colors and soft contact shading. Standard `KHR_mesh_quantization` reduces the file to 5,875,772 bytes. Uniform position quantization is approximately 0.11 mm per step in the source units.
- `kitchen-laptop.webp` and `kitchen-cookbook.webp`: the actual Noms recipe editor capture and sourdough recipe spread, retained on the approved smaller props.
- `sams-kitchen.webp`: 960px still image used in reading mode, reduced motion and graphics fallbacks.
- `kitchen-budget.json`: generated budget/provenance record.
- `kitchen-geometry-audit.json`: per-object source/export triangle counts. Curved props such as the wok, serving plates, cups and stool seats retain their evaluated source geometry.

## Rebuild

Requires Blender 4.3.2, Python 3 and Pillow. Keep `assembly-manifest.json` beside the revision 13 native master and the approved review PNGs in the same source directory. The approved master is read without modifying or saving it. Export to a temporary directory before copying completed outputs here so the app watcher cannot read a partial asset.

```sh
blender -b /path/to/revision-13/sams-kitchen-recipe-manager.blend --threads 5 \
  --python tools/about/export_kitchen.py -- /tmp/noms-kitchen-web
python tools/about/pack_kitchen.py /tmp/noms-kitchen-web
cp /tmp/noms-kitchen-web/sams-kitchen.glb assets/about/sams-kitchen-detailed.glb
cp /tmp/noms-kitchen-web/{kitchen-budget.json,kitchen-geometry-audit.json} assets/about/
python tools/about/prepare_kitchen_images.py /path/to/revision-13 assets/about
```

The exporter preserves radial/rim geometry, uses gentler simplification on other surfaces, evaluates rounded edges, joins the diorama while retaining assembly identity per face and bakes subtle contact shading into the material palette at mesh corners. The packer removes unused attributes and quantizes positions, normals and colors. Manganis may re-encode the print images during bundling; verification compares their decoded pixels with the approved source artwork. The two printed surfaces retain quantized UVs and reference local WebP images at runtime. Assembly metadata stores group names, world-space pivots, incoming offsets, and native start/end frames in glTF Y-up coordinates. The app reads this deliberately small GLB contract in Rust; arbitrary third-party GLBs are not supported by this loader.

## Runtime

`src/pages/about_tour` ports the approved first-person Rust/WebGL prototype into the Dioxus About route. Semantic HTML contains all four complete chapters before enhancement. The camera and floating panels share the same projection, scoped beneath the existing app navbar. The kitchen is fetched from the local Manganis asset URL at the opening stop after WebGL enhancement. The 3D tour now loads the kitchen and its print textures on entry, including direct chapter links. A loading curtain locks document scrolling and makes the covered tour controls inert until all assets are uploaded, every kitchen group has been rendered once with GPU completion, and an additional frame has presented. It then opens automatically at the requested chapter. Other routes, initial reading view and reduced motion do not fetch the model or print textures.

Mobile starts with ambient drift paused. Camera travel and reversible kitchen assembly still follow native scrolling. The opening waypoint is 320svh; later steps are 160svh. The first extra segment holds the camera while frames 40–160 assemble appliances, island, food, recipe book and laptop; the model settles before forward travel begins. The floor, room and counters are complete at first paint so the opening starts with an intact architectural shell. Ambient pause stops continuous rendering without disabling scroll input. Pause/read controls, reduced-motion changes, WebGL/context-loss fallbacks and route unmounting stop rendering or dispose resources. Restoring graphics rebuilds the scene and reloads the kitchen if needed. Reading mode uses the lazy still image.

The loader preserves linear glTF vertex colors until fragment lighting. The kitchen uses neutral studio fill and an AgX tone-mapping approximation, followed by sRGB display encoding, to bring its terracotta, peach, cream and oak closer to the Blender master. The tour's green atmospheric fog does not tint the kitchen. This remains a lightweight web lighting approximation rather than the master's Cycles render. The other tour objects keep their existing lighting. See `THIRD_PARTY_LICENSES.txt` for the tone-mapping attribution.

The curtain offers “Read the story instead” throughout loading. Asset or graphics failure, or a 25-second loading deadline, switches to the readable page and releases scrolling. Route exit cancels the deadline; graphics restoration and re-entering 3D warm the renderer again.

## Checks

```sh
cargo fmt --all -- --check
cargo check --locked --target wasm32-unknown-unknown --no-default-features --features web
cargo test --locked --no-default-features --features server pages::about
cargo test --locked --no-default-features --features server pages::marketing_contract_tests
```

The GLB contract test checks the delivered mesh/file budget, finite geometry preservation of all 18 assembly groups and both printed UV surfaces, and preservation of the color palette. Browser verification should cover all four stops, opening assembly and forward/backward scrolling, creator panel scrolling, opening/direct-fragment loading, mobile idle rendering, route exit/re-entry, reduced motion, graphics and image failure, context recovery, and desktop/mobile screenshots. Do not commit browser validation screenshots to the application or PR.
