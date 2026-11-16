# Noms village — outdoor walking preview (revision 6)

[Download noms-village-mobile.zip](https://github.com/SamKoCPSC/Noms/raw/refs/heads/preview/terraced-village-download/noms-village-mobile.zip)

1. Download the ZIP above (about 71 MiB).
2. Extract **OPEN-VILLAGE-MOBILE.html**.
3. Open that HTML in Chrome. All twelve models are inside that file; there is no neighboring assets folder to load.

If you use **Code → Download ZIP**, extract the repository archive, then extract the `noms-village-mobile.zip` inside it.

Revision 2 expands the spacing between locations by 25%, adds continuous ground and pathside trees and benches, and follows paths at walking height with forward-looking turns. The preview starts at the kitchen; use **Walk tour** to start, or **Overview** to inspect the layout.

Revision 3 adds connected entrances at all twelve stops: varied paving, landings, steps and low herb planting. The cabin approach stays on the grassy bank, clear of its stream. Room geometry and the walking route are preserved.

This single-file format replaces the earlier multi-file download, which could show “Failed to fetch” when Android opened its HTML through a `content://` file viewer without access to neighboring model scripts.

Geometry bytes and texture dimensions are unchanged. The download uses WebP quality 95 compression for preview textures; original production assets and Blender masters remain unchanged. Conversion receipts are included in this branch.

The scene remains a large twelve-model layout study. Loading all models together may require substantial phone memory. Terrain and paths are provisional, and shared lighting has not been rebaked.

Source and Blender world: [PR #89](https://github.com/SamKoCPSC/Noms/pull/89).

Validation: the single-file HTML loads all twelve models in portrait browser testing with additional HTTP/HTTPS requests blocked. Direct Android `content://` launching is not available in the test environment and still needs checking on the device.

Revision 2 checks: all twelve stops, 2,211 path samples clear of model footprints at constant terrain clearance, next-button movement, and single-file loading with external requests blocked. The native Blender camera follows the same route.

Revision 3 checks also cover all twelve entrance connections, neighboring-plot clearance and camera clearance from the new additions.

Revision 4 enriches landscape colors, adds mixed trees, shrubs, boulders and meadow planting, and raises the walking camera from 3.8 to 4.6 world units. Additional details are batched to reduce draw calls; route and arrival sightlines stay clear.

Revision 5 adds eight colorful cottages with porches and mailboxes, eleven Noms-themed street names and destination signs, four pedestrian road signs, eleven lantern posts and a village welcome board. The original locations and raised walking route are preserved. Sign lettering is geometry and is also saved in Blender. The Droid Sans font notice is embedded in the HTML and included in the ZIP.

Revision 6 gives world surfaces a matte cartoon finish, bevels cottages and street furniture, rounds sign and door corners, and softens path shoulders. The native Blender world preserves these edge normals. The room assets and camera route remain unchanged.
