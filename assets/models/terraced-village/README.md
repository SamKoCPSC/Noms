# Terraced village — layout and camera study

A separate review prototype using all twelve actual About-tour locations in the
existing story order. The route curves around residential and work courtyards,
crosses the central market, and climbs toward the cabin and communal garden.
It does not replace the application's About page.

The [layout manifest](layout.json) is shared by the browser and Blender builders.
Each location has its own position, elevation, yaw and uniform footprint scale.
The original source files are never saved over. No furniture or room geometry
is redesigned. Blender retains each assembled source as a separate collection,
including its original shader data and the current Penguini placements.

## Review files

- [Portable native world](models/terraced-village-blockout.blend)
- [Source receipts](models/terraced-village-blockout.sources.json)
- [Browser overview](previews/overview.png)
- [Example kitchen arrival](previews/kitchen.png)
- [Central market arrival](previews/market.png)
- [Recorded camera poses](source/camera-poses.json)
- [Plot-avoiding path waypoints](source/paths.json)

In Blender, `Village overview — static` is the initial camera. Select
`Village review camera` as the active scene camera to inspect the keyed tour.
Timeline markers identify all twelve stops. The browser is the primary motion
review: its eased camera curves and framing respond to the viewport, while the
Blender track records the stop poses and raised travel midpoints.

## What remains provisional

The browser copies the existing baked GLBs byte-for-byte, retaining their current
lighting. The native world uses the original Blender sources, including the three
recently restored masters, under a provisional shared sky and sun. These two
lighting treatments intentionally have not been reconciled or rebaked yet.

Terrain pads, connecting paths, river profile, backdrop and sky are blockout
geometry. Browser paths have flat bridge decks and supports; the native study
uses curves for those same path waypoints, ready for terrain detailing. Final
stairs, ramps, collision-safe walkways and landscaping follow layout approval.
The route is a flying camera route, not a pedestrian navigation simulation.

The browser holds all twelve scenes for overview/orbit review and preserves full
textures and triangle counts. This is a desktop-oriented review, not the final
streaming or mobile performance implementation. Playback starts only on request;
reduced-motion preference removes automatic jumps between manually chosen stops.

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

Blender sources and production GLBs have SHA-256 receipts. Browser preview
copies are byte-identical to their source GLBs; the packaging step does not
simplify meshes, resize textures or change materials.

## Verification

The browser review visited all twelve stops, exercised playback and a mid-flight
view, and checked portrait resizing without JavaScript errors. The embedded HTML
also loaded all twelve models without requesting external resources. The managed
verification browser disallows `file://` navigation, so its self-contained HTML
was tested over local HTTP; a direct local-file launch was not verified there.
All twelve archived master hashes remain unchanged.
The saved native world also reopened successfully with 24,226 source objects,
twelve camera-stop markers, no external Blender libraries and no unpacked image
dependencies. See [native verification](source/native-verification.json).

```sh
python tools/about/village/check-preview.py http://127.0.0.1:8766 /tmp/village-check
blender -b -t 2 --disable-autoexec --python-exit-code 1 --python tools/about/village/check-master.py -- \
  assets/models/terraced-village/models/terraced-village-blockout.blend /tmp/village-master-check.json
```
