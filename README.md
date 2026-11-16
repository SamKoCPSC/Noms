# Noms village — connected stone paths preview (revision 8)

[Download noms-village-mobile.zip](https://github.com/SamKoCPSC/Noms/raw/refs/heads/preview/terraced-village-download/noms-village-mobile.zip)

1. Download the ZIP.
2. Extract **OPEN-VILLAGE-MOBILE.html**.
3. Open that HTML in Chrome. All twelve locations, the landscape and viewer code are embedded; there is no neighboring assets folder to load.

If you use **Code → Download ZIP**, extract the repository archive, then extract the `noms-village-mobile.zip` inside it.

Paths and entrance forecourts now form one connected stone network, with softly beveled limestone pieces. Intersections share the same surface height, eliminating the overlapping and lifted strips in the earlier preview. Ground and landscape shadows have been rebaked.

The complete village shares one warm sun and a brighter sky fill, with baked shadows across the rooms, paths, planting and cottages. Matte surfaces and rounded edges are retained. The camera follows the existing paths at walking height and turns toward each location on arrival. Use **Walk tour**, the previous/next buttons or the location menu; **Overview** shows the whole village.

This remains a separate review preview. The production About page and original room masters are unchanged. The full editable Blender world, baked assets, rebuild tools and verification receipts are preserved in [PR #89](https://github.com/SamKoCPSC/Noms/pull/89).

The mobile package uses WebP quality 91 at the source texture dimensions, and verifies geometry bytes against its new shared-lighting GLBs. Those derivatives preserve native geometry; some contain more detail than the older browser models. Static shadows need no real-time shadow maps. Glass and metal retain view-dependent shading with an approximate outdoor reflection environment.

All twelve scenes remain resident for overview review. The single-file preview is checked in desktop and portrait browser layouts with additional HTTP/HTTPS requests blocked. Direct Android `content://` launching and actual-device performance still require device review.

The Droid Sans font notice and viewer licenses are included in the ZIP and embedded viewer. Package conversion receipts are also stored in this branch.

## Earlier lighting comparison

[Download the original kitchen comparison](https://github.com/SamKoCPSC/Noms/raw/refs/heads/preview/terraced-village-download/noms-kitchen-lighting-study.zip). Extract `OPEN-KITCHEN-LIGHTING.html` and use **Current lighting / Shared sunlight** to compare the same camera view. This retains the earlier bounded kitchen trial; the full village above uses brighter ambient fill.
