# NOMS-026: Guided About village and first-person exploration

**Status:** ✅ Done — guided and free exploration baselines delivered

**Parent:** [NOMS-013](../implementation-plans/NOMS-013-marketing-pages-3d-experience.md)

**Delivered through:** 2026-11-18 ([PR #103](https://github.com/SamKoCPSC/Noms/pull/103))

The public About page guides visitors through twelve story stops in one 3D
village. The current tour opens on a wide view, then approaches the kitchen.
Buttons and keyboard navigation travel along the path. Scrolling moves through
the village continuously. Desktop puts the story beside the featured location
and mobile puts it above the scene.

- **Explore village** is available throughout the tour. Visitors can move
  anywhere within the fence, then resume at their previous tour position,
  including between stops. The fence and movement bounds use the same outline.
- Desktop movement starts and stops immediately. WASD walks at 5 m/s, Shift
  sprints at 10 m/s, Space jumps once per press, and holding Left Ctrl crouches
  at 2.5 m/s. The walking flow also has touch movement and drag-to-look controls.
- Rust controls navigation, loading, focus and cleanup. Three.js renders the
  Blender-derived village with baked lighting and view-dependent glass. The
  reading experience provides a fallback for reduced motion, rendering failure
  and short viewports, and an immediate escape from loading.

The [tour implementation record](../implementation-plans/NOMS-026-about-village-tour.md)
tracks the sequence of renderer, camera, scene and controls changes. The latest
movement PR records 14 camera/movement tests and Chromium verification of
jumping, landing and crouching with the full village. Physical-device, assistive
technology and release performance acceptance remain under NOMS-013/020/022.
