# About tour models

Web derivative of Sam's approved **revision 13** Blender kitchen. The editable master and source renders remain on the [model review page](https://chatgpt.com/space/page_60c2cf7e836c8191a4c07002a7fdfc0c); this directory contains the production derivative, not the native master.

- `sams-kitchen-detailed.glb`: 187,949 triangles, 18 assembly groups and 20 draw calls (two additional print surfaces). Standard `KHR_mesh_quantization` packs geometry, material colors and atlas/print UVs; a custom `_METALLIC` scalar preserves the original material's metal factor; `kitchen-budget.json` records the delivered byte size. Uniform position quantization is approximately 0.11 mm per step in the source units.
- `kitchen-laptop.webp` and `kitchen-cookbook.webp`: the actual Noms recipe editor capture and sourdough recipe spread, retained on the approved smaller props.
- `kitchen-surface-color.webp`: 2048² lossless atlas containing sRGB albedo in RGB and assembly-local cavity shading in alpha. It contains no scene lighting or cast shadows.
- `kitchen-surface-normal.webp`: 2048² lossless, opaque RGB atlas containing octahedral object-space normals in RG and roughness in B. Numerical channels bypass display transforms and sRGB decoding. The converter verifies that normal quantization adds less than one degree of angular error.
- `sams-kitchen.webp`: 960px still image used in reading mode, reduced motion and graphics fallbacks.
- `kitchen-budget.json`: generated budget/provenance record.
- `kitchen-geometry-audit.json`: per-object source/export triangle counts. Curved props such as the wok, serving plates, cups and stool seats retain their evaluated source geometry.

## Rebuild

Requires Blender 4.3.2, Python 3, NumPy and Pillow. Keep `assembly-manifest.json` beside the revision 13 native master and the approved review PNGs in the same source directory. The approved master is read without modifying or saving it. Export to a temporary directory before copying completed outputs here so the app watcher cannot read a partial asset.

```sh
blender -b /path/to/revision-13/sams-kitchen-recipe-manager.blend --threads 5 \
  --python tools/about/export_kitchen.py -- /tmp/noms-kitchen-web --surface-materials
python tools/about/pack_kitchen.py /tmp/noms-kitchen-web
python tools/about/prepare_kitchen_surfaces.py /tmp/noms-kitchen-web assets/about
cp /tmp/noms-kitchen-web/sams-kitchen.glb assets/about/sams-kitchen-detailed.glb
cp /tmp/noms-kitchen-web/{kitchen-budget.json,kitchen-geometry-audit.json} assets/about/
python tools/about/prepare_kitchen_images.py /path/to/revision-13 assets/about
```

The exporter preserves radial/rim geometry, uses gentler simplification on other surfaces, evaluates rounded edges, and joins the diorama while retaining assembly identity per face. The `--surface-materials` mode bakes the master’s unlit albedo, procedural and texture normals, roughness, and local cavities into two shared atlases. Cavity shading is limited to 0.045 source units within a rigid assembly; the independently unfolding walls receive none. Original material metallic factors remain separate per-vertex scalars. The packer removes unused attributes and quantizes positions, normals, colors and UVs while retaining `_METALLIC`. The loader packs that scalar into the otherwise unused GPU vertex-color alpha channel. The laptop and cookbook keep their readable prints. Assembly metadata stores group names, world-space pivots, incoming offsets, native start/end frames in glTF Y-up coordinates, and texture indices. The app reads this deliberately small GLB contract in Rust; arbitrary third-party GLBs are not supported by this loader.

## Office

The second stop uses Sam's approved **revision 10** office from the [office model review](https://chatgpt.com/space/page_65be16eb77f08191a6d3faeb22138385). The native master remains separate from the application. Its SHA-256 is recorded in `office-budget.json`.

- `sams-office.glb`: 240,691 triangles, 16 assemblies, 24 draw calls and 7,341,688 bytes. Quantized position steps are approximately 0.095 mm in source units.
- `office-editor-left.webp`, `office-editor-right.webp`, `office-rtx4090.webp`, and `office-intel-core.webp`: the approved editor and packaging artwork on the actual modeled print surfaces.
- `sams-office.webp`: the approved 960px still for reading mode, reduced motion and graphics fallbacks.
- `office-geometry-audit.json`: per-object evaluated and exported triangle counts. All chair-weave strands and separate keycaps remain present. The corrected thumb buttons and their surrounding recesses are preserved without decimation.

The exporter evaluates the master, simplifies in an isolated scene, and exports only that scene. Contact shading excludes glazing so the PC interior remains visible. Transparent surfaces retain opacity metadata and render after opaque geometry without writing depth. The office uses the shared matte lighting and AgX display treatment described below; printed screens and packaging retain their readable artwork.

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

The story is split into twelve shorter cards, preserving the original paragraphs and repeated category headings. Each card has a unique anchor and its own scene instance, entrance state and shadow cache. The five model assets and their GPU geometry/textures are shared; repeated bedroom and potluck stops do not download or upload duplicate meshes.

The camera renders across the full viewport. Text is a stationary overlay that fades during travel, rather than a clipped viewport that crops the camera. On mobile, content-sized cards are centered in the upper half of the space below the navbar. A separate framing rectangle fits settled models beneath the card (beside it on desktop) without clipping moving geometry. Cards do not capture scroll gestures. Each stop has a half-viewport reading pause followed by 1.5 viewport heights of camera travel. Back/continue links target the individual stops.

The original five-stop camera spline, yaw and easing are retained. Longer routes repeat its four legs with rigid transforms joining the endpoints, so each additional card has its own room along the same forward-moving route. Each room plays its three-second entrance once when approached, including direct fragment entry, and remains assembled on later visits. Hidden tabs, reading mode and the loading curtain suspend playback. Mobile ambient drift starts paused; rendering stops after arrivals settle unless the camera moves.

The existing Penguini loading curtain remains. All five assets are fetched and warmed once before it opens at the requested card. The reading shortcut, reduced motion, graphics failure and short-screen fallback expose normal document content. Leaving the route cancels pending loading and releases rendering resources. The transparent canvas matches the app background. Resize observation and per-frame layout checks keep the initial canvas aspect correct before any scrolling.

Run `scripts/tests/test-about-card-overlay-browser.py --url http://localhost:8080/about` against the local app to check full-viewport projection, centered cards, independent repeated-room arrivals, original camera travel, resize and reduced motion. The existing `test-about-tour-resize-browser.py` also covers late canvas layout changes.

The office at the second stop plays its own three-second entrance the first time the camera approaches from either direction, including direct `#why` entry. The floor and walls enter first; windows, furniture, monitors, PC, chair and accessories follow. Scroll triggers the entrance once and does not scrub or reverse it. The office remains assembled on later visits to the stop. Reading view, hidden tabs and the loading curtain suspend playback; ambient pause does not prevent the entrance. A settled, paused tour stops rendering. Its lazy still appears in the second reading-mode chapter.

The potluck room uses the same one-time, three-second entrance at the philosophy stop, including direct `#community` entry. The floor and walls assemble first; dining and lounge furniture, food, chess and buffet accessories follow as the camera approaches. The room stays assembled on return visits, and its lazy still appears in the philosophy reading-mode cards.

All entrances remap the authored offsets into the open sides of each room: front/right for the kitchen and potluck room, and front/left for the office. Every moving group has an inward horizontal approach with a restrained upward lift. Shared pivots keep prints and glazing attached to their parent assemblies; final positions and scales are unchanged. Geometry-based tests sample all three exported models throughout the entrance to verify that moving bounds never extend farther into either wall than their final mounted position.

All five dioramas use a shared matte finish, room-local key/fill/rim lights and dynamic shadows. Roughness is remapped into 0.72–1.0 and specular highlights are attenuated to 24–38%, retaining slightly more sheen where metallic data is available. Warm colors receive restrained saturation through the shared AgX display treatment. The lighting directions remain attached to each room as the camera moves; assembly normals rotate with their objects. There is no material or lighting swap at the end of an entrance.

The kitchen additionally uses the master's unlit albedo, normal and roughness atlases to restore pastry layers, subtle plaster and wood texture, and material variation on enamel, ceramic and metal. Its shader converts object-space normals from Blender Z-up to glTF Y-up. No assembled-room shadows are baked into those atlases. The other four approved GLBs keep their authored vertex colors, normals and existing mild vertex contact shading; this change does not rebuild their geometry or add material atlases. Monitor screens, posters, windows and packaging keep their readable artwork; pastry textures receive the shared matte lighting. Transparent PC glazing and drink glass render last without depth writes and do not cast opaque shadows.

Each visible room has its own packed-depth WebGL1 shadow pass, using the same assembly poses and ordered entrance reveal as the visible pass. A fixed orthographic light frustum contains every assembly throughout its flight, and nine depth comparisons soften shadow edges. Each filter tap compares at the receiver plane’s depth, with a texel-aware bias, to prevent diagonal self-shadow bands across broad floors and the lawn. Shadows move with the arriving pieces rather than displaying their final silhouettes on empty walls and floors. See `THIRD_PARTY_LICENSES.txt` for the AgX tone-mapping attribution.

The two 2048² surface atlases add approximately 42.7 MiB of RGBA GPU storage including mipmaps, shared across all kitchen assemblies. They add no draw calls or triangles. On unsupported shadow targets, the surface materials still render with ordinary studio lighting.

Shadow targets start at 1024² on mobile (6 MiB of color/depth storage) or 2048² on desktop (24 MiB), subject to hardware limits. Twelve sustained slow visible entry frames lower the current resolution, to a 512² floor (1.5 MiB). Pauses and isolated hitches are excluded. Each map updates only while its assembly poses change; settled shadows remain cached during camera movement and scrolling. Only visible rooms allocate targets: one at a chapter stop, at most two between neighboring chapters (12 MiB mobile / 48 MiB desktop at initial resolutions). Hidden rooms and reading mode release their targets; returning rebuilds the map once without replaying the entrance. Unsupported shadow shaders or incomplete target allocation fall back to the same matte materials and ordinary studio lighting. Reduced-motion reading mode and context recovery retain the existing behavior. Canvas attributes `data-shadow-room`, `data-shadow-targets`, `data-shadow-size` and `data-shadow-updates`, plus per-room size/update attributes, expose allocation and caching for browser verification.

Shadow tests sample all five rooms’ moving bounds against their light frusta and exercise quality limits. Browser checks should compare caster/receiver transforms and reveal values during the entrance, verify cached updates at rest and resource release offscreen, exercise low-quality and allocation-failure paths, and inspect desktop/mobile contact shadows. Performance timings from a software GPU are not representative of real mobile hardware.

The curtain offers “Click here if the page takes too long to load” throughout loading. A collection of 100 playful cooking phrases rotates every three seconds while loading, starting at a random phrase on each entry, with a stable screen-reader status. The phrase timer stops when the curtain closes or the route unmounts. Asset or graphics failure, or a 25-second loading deadline, switches to the readable page and releases scrolling. Route exit cancels the deadline; graphics restoration and re-entering 3D warm the renderer again.

## Checks

```sh
cargo fmt --all -- --check
cargo check --locked --target wasm32-unknown-unknown --no-default-features --features web
cargo test --locked --no-default-features --features server pages::about
cargo test --locked --no-default-features --features server pages::marketing_contract_tests
```

The GLB contract tests check all five file/mesh budgets, finite geometry, the 18 kitchen, 16 office, 26 potluck, 32 bedroom and 29 garden assemblies, all thirteen printed UV surfaces, transparent PC panels and potluck glazing, preserved mouse controls, curved furniture and dishes, carved chess pieces, the kitchen surface UVs and metallic range, and the color palette. The bounded loader permits at most 64 primitives, five print indices and 400,000 triangles per model. Browser verification should cover all twelve stops, automatic opening assembly without scrolling, settled geometry during forward/backward scrolling, creator card transitions, opening/direct-fragment loading, one-time office, bedroom, potluck and garden approach/direct-entry/reverse-approach arrivals, mobile idle rendering, route exit/re-entry, reduced motion, graphics and image failure, context recovery, and desktop/mobile screenshots. Do not commit browser validation screenshots to the application or PR.

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

## Garden cookout

The fifth chapter, “The Noms Philosophy” (`#gather`), uses the approved **revision 6**
[garden model](https://chatgpt.com/space/page_7c6bf3f7c53081919c1588f851054446).
It retains the shared meals, beer picnic table, enlarged centered spikeball net,
sports equipment at the side, left fence and two striped lawn chairs.

The web derivative contains 394,185 triangles, 29 assemblies and 32 draws, with
one pastry texture. Quantization reduces the GLB to 12,892,928 bytes; the native
master stays unchanged. Rounded dishes, bottles and sports equipment retain their
evaluated geometry. `garden-budget.json` records provenance and
`garden-geometry-audit.json` records the per-object simplification.

Ground, house and fences are assembled immediately. The remaining groups arrive
once on approaching the fifth stop, through the open front (+Z), keeping clear
of both side fences. They remain settled when revisiting the chapter. Loading
includes the garden's geometry, texture and GPU warm-up; reader, reduced-motion
and graphics-failure modes use the approved Blender still.

```sh
blender -b /path/to/revision6/garden-v6.blend --threads 5 \
  --python tools/about/export_garden.py -- /tmp/noms-garden-web
python tools/about/pack_kitchen.py /tmp/noms-garden-web garden
python tools/about/prepare_garden_images.py /path/to/revision6 /tmp/noms-garden-web assets/about
cp /tmp/noms-garden-web/{sams-garden.glb,garden-budget.json,garden-geometry-audit.json} assets/about/
```

Keep `assembly-manifest.json` and `garden-v6-clean.png` beside the master. The
geometry and entrance tests validate the delivered asset, including fence
clearance, group count, texture references and the existing triangle ceiling.
