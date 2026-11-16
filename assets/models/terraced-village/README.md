# Village — outdoor walking study

A separate review prototype using all twelve actual About-tour locations in the
existing story order. Revision 2 expanded the model spacing by 25% in both horizontal
axes, kept their sizes intact, and replaced the elevated display plinths with
continuous ground, low thresholds, paths, benches, scattered trees and a river.
Revision 3 connects all twelve walking stops to their locations with forecourts,
landings, two or three broad steps, and low herb planting. Brick, stone, timber
and gravel treatments vary by location. The cabin entrance is offset onto the
grass, clear of its stream. The approved walking route and room models remain
unchanged. It does not replace the application's About page.

The walking camera is raised from 3.8 to 4.6 world units. The current layout enriches
the landscape with 66 additional trees, 95 bushes, 81 rock groups and 128 meadow
patches. Seeded placement leaves the route, entrances, river and arrival sightlines
clear. The 1,895 small detail parts are batched into 33 meshes to reduce draw calls.
Richer greens, blue water and warm paths complement the room models. Revision 7
replaces this earlier live landscape lighting with the shared daylight bake.
Original model assets remain unchanged.

Revision 5 adds eight colored cottages with pitched roofs, porches, window herb
boxes and mailboxes, plus eleven street signs, four pedestrian road signs, eleven
lantern posts and a village welcome board. Street names include Noms Lane, Recipe
Row, Whisk Way, Pantry Place, Simmer Street, Supper Lane, Sourdough Rise, Market
Mews, Gathering Green, Campfire Crescent and Potluck Parade. Planting is placed
after buildings and signs to keep their footprints clear. Signs use geometric
Droid Sans lettering, preserved in Blender and embedded in the offline preview;
the [Apache 2.0 font notice](../../../tools/about/village/DROID-FONT-NOTICE.txt)
accompanies both preview formats and is stored in the Blender file.

Revision 6 gives generated world surfaces a matte, cartoon finish: full roughness,
low specular response and no clearcoat or metallic sheen. Cottages, roofs, benches,
steps, planters, mailboxes and signs have real bevels; thin doors and signs also
have rounded face corners. Paths have softly rolled shoulders. Geometry stays
inside its original bounds, preserving the walking route and room placement.
Exported custom normals preserve the softened edge shading in the native world.

Revision 7 extends the approved kitchen lighting study to the complete village.
All twelve rooms and the landscape share one warm sun, a brighter sky fill and
baked shadows. The [daylight assets and rebuild instructions](daylight/README.md)
preserve the original source models and the approved walking route. Ground uses
four lighting atlases; narrow rounded landscape surfaces use baked vertex colors.

Revision 8 replaces overlapping path strips with one connected matte stone
network. Beveled limestone paving and a continuous grout bed follow the terrain
and blend smoothly into the entrance forecourts. Ground and landscape shadows
are rebaked with the same bright settings. All twelve room exports are retained.

Revision 9 adds folded grass blades, shaped wildflowers and subtle ground color
variation inspired by a stylized meadow. Tree crowns, shrubs and evergreen tiers
have layered ridges instead of smooth spherical foliage. Planting stays clear of
paths, entrances and the river. Shared outdoor lighting is rebaked; the rooms and
camera route stay unchanged.

Revision 10 makes the meadow denser: 28,080 tufts, more evenly spread across the
open grass, with 60% more blades. Flower patches and ridged foliage are retained,
and the stone paths and entrances stay clear. Outdoor daylight is rebaked.

The [layout manifest](layout.json) is shared by the browser and Blender builders.
Each location has its own position, elevation, yaw and uniform footprint scale.
The original source files are never saved over. Furniture and room geometry stay
intact. Blender retains each source as a separate native collection, including
its shaders and the current Penguini placements.

## Review files

[Revision 10 denser meadow preview ZIP](https://github.com/SamKoCPSC/Noms/raw/c42f31bb79726f967b1a447856c5dfb9442db31d/noms-village-mobile.zip)
(about 98.7 MiB). Extract and open `OPEN-VILLAGE-MOBILE.html`.

[Earlier revision 6 archive](https://github.com/SamKoCPSC/Noms/raw/c418c7ba978a1dba970abc572a320e0cee43c0de/noms-village-mobile.zip).

- [Meadow and foliage detail](daylight/previews/meadow-detail.png)
- [Repaired library junction](daylight/previews/stone-junction.png)
- [All twelve locations](daylight/previews/all-locations.png)
- [Shared-daylight native world](daylight/models/terraced-village-daylight.blend)
- [Original matte layout world](models/terraced-village-blockout.blend)
- [Source receipts](models/terraced-village-blockout.sources.json)
- [Overview](daylight/previews/overview.png)
- [Kitchen arrival](daylight/previews/kitchen.png)
- [Bakery entrance](daylight/previews/bakery.png)
- [Cabin arrival](daylight/previews/cabin.png)
- [Walking toward the library](daylight/previews/walk-library.png)
- [Mobile arrival](daylight/previews/mobile.png)
- [Camera stops](source/camera-poses.json)
- [Walking camera samples](source/walking-camera.json)
- [Shared environment geometry](source/world-environment.json)
- [Entrance checks](source/entrance-verification.json)
- [Landscape checks](source/landscape-verification.json)
- [Neighborhood checks](source/neighborhood-verification.json)
- [World material and bevel checks](source/style-verification.json)

The browser starts at the first arrival, at walking height. **Walk tour** follows
continuous paths around the model footprints, looks ahead during travel and turns
toward each location on arrival. Previous/next follow the same route. Distant stop
jumps and return from overview use a brief fade. **Overview** remains available
for inspecting the layout. There is no head bob or automatic playback.

The camera stays 4.6 world units above the terrain, scaled to the existing room
models and their thick floor bases. Travel speed is 2.6 world units per second.
Portrait layout adjusts the field of view without raising the camera or changing
its route. Reduced-motion preference makes manual navigation instantaneous.

In Blender, `Village walking camera` is the initial active camera. Its keyed
positions and look directions come from the same sampled route as the browser,
with twelve stop markers. `Village overview — static` provides the elevated view.
The native builder imports the browser's environment geometry so paths, benches,
trees, entrances, planting and ground match; it can reuse the existing master’s source collections.

## Shared-lighting experiment

The [kitchen district study](lighting-study/README.md) retains the original
before/after experiment. The [full-village daylight pass](daylight/README.md)
extends it to every location with brighter ambient fill to keep interiors clear.

## What remains provisional

The normal browser preview copies the new shared-daylight GLBs byte-for-byte.
The mobile single-file preview compresses their textures as documented below.
Opaque surfaces use static baked lighting; glass and reflective metal remain
view dependent and use an approximate outdoor reflection environment.

The village layout remains a prototype. This is a guided outdoor camera route,
not a collision-enabled free-roaming game. Automated checks cover the camera
corridor against expanded model bounds, new entrance geometry and ground height.
Entrance approaches also clear neighboring plots by their half-width. It does not include
pedestrian navigation or physical accessibility simulation.

All twelve scenes remain resident for overview/orbit review. Final streaming,
mobile memory budgets and performance optimization are separate work.

## Rebuild

From the repository root, with npm dependencies installed, Blender 4.3.2 and
Python 3 (Playwright/Chromium for browser verification):

```sh
node tools/about/village/build-preview.mjs /tmp/noms-village-preview
python -m http.server 8766 --bind 127.0.0.1 --directory /tmp/noms-village-preview
```

Open http://localhost:8766. Build a portable HTML and a local-server ZIP:

```sh
python tools/about/village/package-preview.py /tmp/noms-village-preview /tmp/village-downloads
```

For Android file viewers using `content://`, use a single HTML file with all model
data embedded. A multi-file HTML package can fail because the viewer cannot read
neighboring files. Generate the smaller single-file variant with:

```sh
node tools/about/village/build-mobile-preview.mjs /tmp/noms-village-preview /tmp/village-mobile
```

This writes `OPEN-VILLAGE-MOBILE.html` and a conversion receipt. It uses WebP
quality 76 textures at their original dimensions and verifies every copied
geometry buffer range against the source. It does not change production assets
or Blender masters. Zip the HTML for distribution. All twelve scenes remain
resident, so this packaging fix does not establish phone memory/performance
budgets. Android-specific launching must still be checked on a real device.

Prepare native scene libraries without modifying any master:

```sh
python - <<'PY'
import json, subprocess
from pathlib import Path
for s in json.loads(Path('assets/models/terraced-village/layout.json').read_text())['stops']:
    subprocess.run(['blender','-b','-t','2','--disable-autoexec','--python-exit-code','1','--python',
                    'tools/about/village/prepare-master.py','--',s['id'],
                    '/tmp/village-native'],check=True)
PY
cp assets/models/terraced-village/source/camera-poses.json /tmp/village-native/
cp assets/models/terraced-village/source/paths.json /tmp/village-native/
blender -b -t 2 --disable-autoexec --python-exit-code 1 --python tools/about/village/build-master.py -- \
  /tmp/village-native /tmp/terraced-village-blockout.blend
```

Blender sources and production GLBs have SHA-256 receipts. Normal browser preview
copies are byte-identical to their shared-lighting GLBs. Mobile packaging
compresses textures without resizing them or simplifying geometry.

## Verification

The browser review visits all twelve stops, checks playback and portrait framing,
and samples all eleven connecting routes at 201 points each. Camera samples must
stay above the terrain, clear the expanded model footprints and join continuously.
Entrance checks verify all twelve connections start at their actual camera stops.
Floor heights are raycast from the source geometry; `entrance.edgePosition` can
move an entrance along its open edge to avoid a fixed feature.
Source hashes, geometry conversion receipts and native reopen checks are retained.
Direct Android `content://` launching is not available in the managed browser;
the single-file package is tested over localhost with additional network requests
blocked.

```sh
python tools/about/village/check-preview.py http://127.0.0.1:8766 /tmp/village-check
blender -b -t 2 --disable-autoexec --python-exit-code 1 --python tools/about/village/check-master.py -- \
  assets/models/terraced-village/models/terraced-village-blockout.blend /tmp/village-master-check.json
```
