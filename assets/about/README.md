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

The exporter evaluates the master, simplifies in an isolated scene, and exports only that scene. Contact shading excludes glazing so the PC interior remains visible. Transparent surfaces retain opacity metadata and render after opaque geometry without writing depth. All rooms use the same scene-linear lighting and display treatment.

```sh
blender -b /path/to/office-revision-10/sams-project-room.blend --threads 5 \
  --python tools/about/export_office.py -- /tmp/noms-office-web
python tools/about/pack_kitchen.py /tmp/noms-office-web office
cp /tmp/noms-office-web/sams-office.glb assets/about/
cp /tmp/noms-office-web/{office-budget.json,office-geometry-audit.json} assets/about/
python tools/about/prepare_office_images.py /path/to/office-revision-10 assets/about
```

## Potluck room

The philosophy stop uses the approved **revision 6** communal dining and living room from the [potluck model review](https://chatgpt.com/space/page_e5800ea484448191ab66357e10cd9472). The native Blender master remains separate and unchanged; `potluck-budget.json` records its SHA-256.

- `sams-potluck.glb`: 240,968 triangles, 26 assemblies, 34 draw calls and 8,587,256 bytes. Quantized position steps are approximately 0.13 mm in source units.
- `potluck-window.webp`, `potluck-tv.webp`, `potluck-pastry.webp`, `potluck-chips-salt.webp`, and `potluck-chips-bbq.webp`: the approved window, television, croissant and snack packaging artwork.
- `sams-potluck.webp`: the approved 960 × 720 still for reading mode, reduced motion and graphics fallbacks.
- `potluck-geometry-audit.json`: per-object evaluated/exported triangle counts. Curved furniture, serving dishes and carved chess pieces retain their evaluated geometry. The chess position remains the approved Deep Blue–Kasparov 1997 game 6 position after 8.Nxe6.

The exporter reads the master and adjacent `assembly-manifest.json`, evaluates geometry and bakes contact shading in an isolated scene. Glazing is excluded from contact shading and retains its opacity. Furniture layout, food placement and native assembly pivots remain unchanged.

```sh
blender -b /path/to/potluck-revision-6/noms-potluck-room.blend --threads 5 \
  --python tools/about/export_potluck.py -- /tmp/noms-potluck-web
python tools/about/pack_kitchen.py /tmp/noms-potluck-web potluck
python tools/about/prepare_potluck_images.py /path/to/potluck-revision-6 \
  /tmp/noms-potluck-web /tmp/noms-potluck-web
cp /tmp/noms-potluck-web/sams-potluck.glb assets/about/
cp /tmp/noms-potluck-web/{potluck-budget.json,potluck-geometry-audit.json} assets/about/
cp /tmp/noms-potluck-web/{potluck-*.webp,sams-potluck.webp} assets/about/
```

## Runtime

`src/pages/about_tour` ports the approved first-person Rust/WebGL prototype into the Dioxus About route. Semantic HTML contains all four complete chapters before enhancement. The camera and floating panels share the same projection, scoped beneath the existing app navbar. All four models and their twelve print textures load from local Manganis URLs on entry, including direct chapter links. A loading curtain locks document scrolling and makes covered controls inert until all assets are uploaded, all four rooms have rendered with GPU completion, and an additional frame has presented. It opens at the requested chapter. Other routes, initial reading view and reduced motion do not fetch model or print assets. Leaving the loading tour invalidates pending work before another model can start downloading or upload to an obsolete renderer.

After the loading curtain closes, the kitchen plays a three-second fly-in once per route visit, independently of scrolling. Frames 40–160 assemble appliances, island, food, recipe book and laptop, then all parts remain in their final positions. The floor, room and counters are complete at first paint. Mobile starts with ambient drift paused but still plays the intro; rendering stops when it finishes unless the camera is moving. Hidden tabs suspend playback. Scrolling or switching to reading view never resets it, and direct links to later chapters skip the unseen intro. All chapter steps are 160svh, with no extra opening assembly segment. Native scrolling continues to move the camera between chapters. Ambient pause stops continuous rendering after the intro without disabling scroll input. Pause/read controls, reduced-motion changes, WebGL/context-loss fallbacks and route unmounting stop rendering or dispose resources. Restoring graphics rebuilds the scene and reloads all four rooms if needed. Reading mode uses the lazy still image.

The office at the second stop plays its own three-second entrance the first time the camera approaches from either direction, including direct `#why` entry. The room shell is present immediately; windows, furniture, monitors, PC, chair and accessories arrive using the approved assembly timing. Scroll triggers the entrance once and does not scrub or reverse it. The office remains assembled on later visits to the stop. Reading view, hidden tabs and the loading curtain suspend playback; ambient pause does not prevent the entrance. A settled, paused tour stops rendering. Its lazy still appears in the second reading-mode chapter.

The potluck room uses the same one-time, three-second entrance at the philosophy stop, including direct `#community` entry. The room shell is present immediately; dining and lounge furniture, food, chess and buffet accessories arrive as the camera approaches. The room stays assembled on return visits, and its lazy still appears in the fourth reading-mode chapter.

All entrances remap the authored offsets into the open sides of each room: front/right for the kitchen and potluck room, and front/left for the office. Every moving group has an inward horizontal approach with a restrained upward lift. Shared pivots keep prints and glazing attached to their parent assemblies; final positions and scales are unchanged. Geometry-based tests sample all three exported models throughout the entrance to verify that moving bounds never extend farther into either wall than their final mounted position.

The loader preserves linear glTF vertex colors until fragment lighting. The kitchen uses neutral studio fill and an AgX tone-mapping approximation, followed by sRGB display encoding, to bring its terracotta, peach, cream and oak closer to the Blender master. The tour's green atmospheric fog does not tint the kitchen. This remains a lightweight web lighting approximation rather than the master's Cycles render. The other tour objects keep their existing lighting. See `THIRD_PARTY_LICENSES.txt` for the tone-mapping attribution.

The curtain offers “Click here if the page takes too long to load” throughout loading. A collection of 100 playful cooking phrases rotates every three seconds while loading, starting at a random phrase on each entry, with a stable screen-reader status. The phrase timer stops when the curtain closes or the route unmounts. Asset or graphics failure, or a 25-second loading deadline, switches to the readable page and releases scrolling. Route exit cancels the deadline; graphics restoration and re-entering 3D warm the renderer again.

## Checks

```sh
cargo fmt --all -- --check
cargo check --locked --target wasm32-unknown-unknown --no-default-features --features web
cargo test --locked --no-default-features --features server pages::about
cargo test --locked --no-default-features --features server pages::marketing_contract_tests
```

The GLB contract tests check all four file/mesh budgets, finite geometry, the 18 kitchen, 16 office, 26 potluck and 32 bedroom assemblies, all twelve printed UV surfaces, transparent PC panels and potluck glazing, preserved mouse controls, curved furniture and dishes, carved chess pieces, and the color palette. The bounded loader permits at most 64 primitives, five print indices and 400,000 triangles per model. Browser verification should cover all four stops, automatic opening assembly without scrolling, settled geometry during forward/backward scrolling, creator panel scrolling, opening/direct-fragment loading, one-time office, bedroom and potluck approach/direct-entry/reverse-approach arrivals, mobile idle rendering, route exit/re-entry, reduced motion, graphics and image failure, context recovery, and desktop/mobile screenshots. Do not commit browser validation screenshots to the application or PR.

## Creator bedroom

The creator stop uses the approved revision 18 bedroom from the [model review](https://chatgpt.com/space/page_dbc8487deee88191aa97de7c374084b6). The wider keyboard desk, enlarged badminton and lounge groups, lowered posters, baker penguin and three tabletop puzzles are retained. The master SHA is recorded in `bedroom-budget.json`.

The architectural shell is present immediately. Furniture arrives before small props, with each printed or transparent surface sharing its parent assembly pivot and timing. The Rust chapter arrival starts near chapter 2, plays once over three seconds, and stays assembled on return. All moving parts enter from the open +X/+Z sides. The actual exported vertices are checked against both wall planes throughout the animation. Reading, reduced-motion and graphics-failure modes use the approved Blender still.

```sh
blender -b /path/to/revision18/sams-corner-cocoon.blend --threads 4 \
  --python tools/about/export_bedroom.py -- /tmp/noms-bedroom-web
python tools/about/pack_kitchen.py /tmp/noms-bedroom-web bedroom
python tools/about/prepare_bedroom_images.py /path/to/revision18 /tmp/noms-bedroom-web assets/about
cp /tmp/noms-bedroom-web/{sams-bedroom.glb,bedroom-budget.json,bedroom-geometry-audit.json} assets/about/
cargo test --locked --no-default-features --features server pages::about_tour
```

The exporter reads `assembly-manifest.json` beside the master and never saves the source scene. The app uses the bounded Rust GLB decoder, without a JavaScript 3D runtime. The bedroom contains 342,007 triangles in 36 draws; the 400,000-triangle ceiling remains per model, not unbounded.
