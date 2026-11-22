# Kitchen district shared-lighting trial

A bounded before/after study of the first kitchen and its surrounding entrance,
path, cottage, trees and planting. The original kitchen shaders and the generated
matte landscape receive one warm sun and a cool ambient sky. Direct shadows,
indirect light and diffuse color are baked together. The kitchen and ground use
three 2048px atlases; narrow landscape surfaces use baked vertex colors to avoid
dark seams on small UV islands.
The first view is the existing village walking camera; the comparison does not
reset the camera when switching lighting.

The full village, original room masters and production About tour are unchanged.
This study covers only the kitchen district, not a completed village-wide rebake.
The bounded landscape has visible outer edges when viewed from above.

The portable HTML includes **Current lighting** and **Shared sunlight** toggles,
three camera views, orbit controls and a short segment of the actual walking
route. Opaque materials are unlit in Three.js because their textures or colors already
contain the lighting and AgX color transform. Glass and metal retain live,
view-dependent shading under an approximate outdoor reflection field; these
surfaces will not match a Cycles render exactly. No real-time shadow maps are used.

## Review files

[Download the portable comparison](https://github.com/SamKoCPSC/Noms/raw/4de3746cc06ab58e6edfc7d006f90754c58c8b00/noms-kitchen-lighting-study.zip)
(about 20 MiB). Extract and open `OPEN-KITCHEN-LIGHTING.html`.

- [Native Blender district](models/kitchen-district.blend)
- [Combined baked model](models/after.glb)
- [Walking camera — shared sunlight](previews/arrival-after.png)
- [Walking camera — current lighting](previews/arrival-before.png)
- [Path and cottage](previews/path-after.png)
- [Mobile view](previews/mobile-after.png)
- [Bake configuration and receipts](source/bake.json)
- [Native verification](source/native-verification.json)
- [Browser verification](source/browser-verification.json)

## Rebuild

From the repository root:

```sh
blender -b -t 3 --disable-autoexec --python-exit-code 1 \
  --python tools/about/village/lighting/prepare.py -- /tmp/kitchen-lighting-study
blender -b -t 3 --disable-autoexec --python-exit-code 1 \
  --python tools/about/village/lighting/bake.py -- /tmp/kitchen-lighting-study 64
blender -b -t 3 --disable-autoexec --python-exit-code 1 \
  --python tools/about/village/lighting/bake-landscape.py -- /tmp/kitchen-lighting-study 64
node tools/about/village/lighting/build.mjs /tmp/kitchen-lighting-study /tmp/kitchen-lighting-preview
blender -b -t 2 --disable-autoexec --python-exit-code 1 \
  --python tools/about/village/lighting/check-native.py -- /tmp/kitchen-lighting-study
python -m http.server 8783 --bind 127.0.0.1 --directory /tmp/kitchen-lighting-preview
```

In another shell:

```sh
python tools/about/village/lighting/check.py \
  http://127.0.0.1:8783/OPEN-KITCHEN-LIGHTING.html /tmp/kitchen-lighting-check
```

The builder embeds all model and script bytes in one HTML, preserves PNG texture
resolution, and verifies that mesh compression removes no triangles. The browser
check blocks subsequent HTTP/HTTPS requests, checks both modes and all three
views, verifies identical comparison cameras, walking movement and portrait
aspect ratio. Android `content://` launching still requires a real-device check.

The native source has packed dependencies and no animations. Source hashes in
`study.json` verify that neither the original kitchen nor the full world was
saved over. Linear EXRs are generated for rebake work; review GLBs contain the
display-ready PNG atlases.
