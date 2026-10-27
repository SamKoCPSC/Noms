# About tour models

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

## Office

The second stop uses Sam's approved **revision 10** office from the [office model review](https://chatgpt.com/space/page_65be16eb77f08191a6d3faeb22138385). The native master remains separate from the application. Its SHA-256 is recorded in `office-budget.json`.

- `sams-office.glb`: 240,691 triangles, 16 assemblies, 24 draw calls and 7,341,688 bytes. Quantized position steps are approximately 0.095 mm in source units.
- `office-editor-left.webp`, `office-editor-right.webp`, `office-rtx4090.webp`, and `office-intel-core.webp`: the approved editor and packaging artwork on the actual modeled print surfaces.
- `sams-office.webp`: the approved 960px still for reading mode, reduced motion and graphics fallbacks.
- `office-geometry-audit.json`: per-object evaluated and exported triangle counts. All chair-weave strands and separate keycaps remain present. The corrected thumb buttons and their surrounding recesses are preserved without decimation.

The exporter evaluates the master, simplifies in an isolated scene, and exports only that scene. Contact shading excludes glazing so the PC interior remains visible. Transparent surfaces retain opacity metadata and render after opaque geometry without writing depth. Both rooms use the same scene-linear lighting and display treatment.

```sh
blender -b /path/to/office-revision-10/sams-project-room.blend --threads 5 \
  --python tools/about/export_office.py -- /tmp/noms-office-web
python tools/about/pack_kitchen.py /tmp/noms-office-web office
cp /tmp/noms-office-web/sams-office.glb assets/about/
cp /tmp/noms-office-web/{office-budget.json,office-geometry-audit.json} assets/about/
python tools/about/prepare_office_images.py /path/to/office-revision-10 assets/about
```

## Runtime

`src/pages/about_tour` ports the approved first-person Rust/WebGL prototype into the Dioxus About route. Semantic HTML contains all four complete chapters before enhancement. The camera and floating panels share the same projection, scoped beneath the existing app navbar. Both models and their six print textures load from local Manganis URLs on entry, including direct chapter links. A loading curtain locks document scrolling and makes covered controls inert until all assets are uploaded, both rooms have rendered with GPU completion, and an additional frame has presented. It opens at the requested chapter. Other routes, initial reading view and reduced motion do not fetch model or print assets. Leaving the loading tour invalidates pending work before another model can start downloading or upload to an obsolete renderer.

After the loading curtain closes, the kitchen plays a three-second fly-in once per route visit, independently of scrolling. Frames 40–160 assemble appliances, island, food, recipe book and laptop, then all parts remain in their final positions. The floor, room and counters are complete at first paint. Mobile starts with ambient drift paused but still plays the intro; rendering stops when it finishes unless the camera is moving. Hidden tabs suspend playback. Scrolling or switching to reading view never resets it, and direct links to later chapters skip the unseen intro. All chapter steps are 160svh, with no extra opening assembly segment. Native scrolling continues to move the camera between chapters. Ambient pause stops continuous rendering after the intro without disabling scroll input. Pause/read controls, reduced-motion changes, WebGL/context-loss fallbacks and route unmounting stop rendering or dispose resources. Restoring graphics rebuilds the scene and reloads both rooms if needed. Reading mode uses the lazy still image.

The office replaces the second stop's book placeholder and keeps its approved assembled pose while the camera moves. Its lazy still appears in the second reading-mode chapter.

The loader preserves linear glTF vertex colors until fragment lighting. The kitchen uses neutral studio fill and an AgX tone-mapping approximation, followed by sRGB display encoding, to bring its terracotta, peach, cream and oak closer to the Blender master. The tour's green atmospheric fog does not tint the kitchen. This remains a lightweight web lighting approximation rather than the master's Cycles render. The other tour objects keep their existing lighting. See `THIRD_PARTY_LICENSES.txt` for the tone-mapping attribution.

The curtain offers “Click here if the page takes too long to load” throughout loading. A collection of 100 playful cooking phrases rotates every three seconds while loading, starting at a random phrase on each entry, with a stable screen-reader status. The phrase timer stops when the curtain closes or the route unmounts. Asset or graphics failure, or a 25-second loading deadline, switches to the readable page and releases scrolling. Route exit cancels the deadline; graphics restoration and re-entering 3D warm the renderer again.

## Checks

```sh
cargo fmt --all -- --check
cargo check --locked --target wasm32-unknown-unknown --no-default-features --features web
cargo test --locked --no-default-features --features server pages::about
cargo test --locked --no-default-features --features server pages::marketing_contract_tests
```

The GLB contract tests check both file/mesh budgets, finite geometry, the 18 kitchen and 16 office assemblies, all six printed UV surfaces, transparent PC panels, preserved mouse controls, and the color palette. Browser verification should cover all four stops, automatic opening assembly without scrolling, settled geometry during forward/backward scrolling, creator panel scrolling, opening/direct-fragment loading, mobile idle rendering, route exit/re-entry, reduced motion, graphics and image failure, context recovery, and desktop/mobile screenshots. Do not commit browser validation screenshots to the application or PR.
