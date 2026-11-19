# NOMS-026: About village implementation record

**Source baseline:** `main` at `e6dea778` (2026-11-18)

**Roadmap item:** [Guided About village and first-person exploration](../issues/NOMS-026-about-village-tour.md)

| Delivery | Result | Merged evidence |
| --- | --- | --- |
| Five-scene tour refinement | Staged entrances, matte lighting, mobile document scrolling and proportions, projection updates on canvas resize. | [#58](https://github.com/SamKoCPSC/Noms/pull/58), [#65](https://github.com/SamKoCPSC/Noms/pull/65), [#68](https://github.com/SamKoCPSC/Noms/pull/68), [#69](https://github.com/SamKoCPSC/Noms/pull/69), [#71](https://github.com/SamKoCPSC/Noms/pull/71) |
| Twelve story cards and new rooms | Short cards over a full viewport, distinct library, supermarket, office and community settings, contemporary kitchen and riverside campsite, and Chef Penguini figures. | [#76](https://github.com/SamKoCPSC/Noms/pull/76), [#78](https://github.com/SamKoCPSC/Noms/pull/78), [#81](https://github.com/SamKoCPSC/Noms/pull/81), [#82](https://github.com/SamKoCPSC/Noms/pull/82), [#84](https://github.com/SamKoCPSC/Noms/pull/84) |
| Loading and keyboard focus | Animated mascot, then focus transfer from loader controls once the active story heading is interactive. The reading escape remains available during loading. | [#74](https://github.com/SamKoCPSC/Noms/pull/74), [#85](https://github.com/SamKoCPSC/Noms/pull/85), [#90](https://github.com/SamKoCPSC/Noms/pull/90) |
| Baked renderer and integrated village | Blender masters and baked derivatives, transparent materials, fully assembled locations, one shared outdoor world and connected camera paths. | [#86](https://github.com/SamKoCPSC/Noms/pull/86), [#87](https://github.com/SamKoCPSC/Noms/pull/87), [#88](https://github.com/SamKoCPSC/Noms/pull/88), [#89](https://github.com/SamKoCPSC/Noms/pull/89) |
| Guided camera refinements | Raised viewpoints, path-facing travel, continuous scrolling and softened turns. The wide opening approach settles over 4.8 seconds and adjacent button travel takes 6.4 seconds at this baseline. | [#92](https://github.com/SamKoCPSC/Noms/pull/92), [#94](https://github.com/SamKoCPSC/Noms/pull/94), [#97](https://github.com/SamKoCPSC/Noms/pull/97), [#99](https://github.com/SamKoCPSC/Noms/pull/99) |
| Free exploration | Look around evolved into first-person walking, then navigation throughout the timber perimeter, with sprint, jump and crouch controls. Resuming restores the original guided pose and scroll position. | [#98](https://github.com/SamKoCPSC/Noms/pull/98), [#100](https://github.com/SamKoCPSC/Noms/pull/100), [#102](https://github.com/SamKoCPSC/Noms/pull/102), [#103](https://github.com/SamKoCPSC/Noms/pull/103) |

The current rendering path uses locally bundled Three.js. The Rust About page
chooses reading or tour mode, restores focus, manages the story and camera, and
disposes the runtime when leaving the route. The art pipeline retains native
sources, lighting provenance and rebuild instructions. Earlier staged room
entrances describe the renderer that preceded the assembled village.

Evidence comes from the linked PRs. Camera and collision tests cover all twelve
locations, the perimeter, gait changes and restoration. Browser checks cover
framing, guided motion, input, reading fallback, failing assets, context loss and
cleanup across the relevant revisions. The [parent plan](NOMS-013-marketing-pages-3d-experience.md) preserves the
earlier acceptance criteria for reference when defining future tasks.
