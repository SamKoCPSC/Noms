# Penguini in 3D

Approved Blender interpretation of Chef Penguini and the 6.6-second excited
jump-and-wave animation. The model preserves the charcoal plumage, cream
heart-shaped face, ivory chef hat, terracotta apron, and golden beak and feet.

The torso and both flippers form one continuous skinned mesh with blended
shoulders. The feet have integrated ankles and flat soles, and the chef hat
fits against the head. Depth and rear details are interpretations of the 2D
illustration.

## Assets

| File | Purpose |
| --- | --- |
| [penguini-excited.glb](penguini-excited.glb) | Character, materials, skeleton, and one synchronized animation clip |
| [source/penguini-excited.blend](source/penguini-excited.blend) | Editable Blender 4.3.2 animation scene with studio cameras and lights |
| [source/base.blend](source/base.blend) | Editable static model with the approved limb connections |
| [source/animate_penguini.py](source/animate_penguini.py) | Rebuilds the animated Blender scene and GLB from the base model |
| [source/excited-reference.json](source/excited-reference.json) | Snapshot of the approved 2D excited clip used as the timing reference |

The GLB contains only the character; the Blender scenes also contain the studio.
Materials require no external textures. Blender coordinates are Z up, facing -Y;
the GLB uses glTF's Y-up convention.

## Animation and editing

Open `source/penguini-excited.blend` and play frames **0–198 at 30 fps**.
The timeline marks the crouch, two hops, landings, wave, recovery, and rest.

- `Penguini` controls the whole-character hop and squash.
- `Head • rotate here` moves the head, face, and hat together.
- `Penguini rig` contains the torso, flipper, and shoulder bones.
- The named `Expression` controls animate blinking, happy eyes, the opening
  beak, and the smile interior.

The easing, two-hop timing, wave rhythm, head tilts, and facial timing follow
the [2D excited clip](../source/clips.json). A small supporting motion is added
to the other flipper. The crouch uses squash about the soles so they stay above
the floor. The animation starts and ends in the same neutral pose, with a gradual
flipper recovery and a resting hold for a smooth loop. Motion is baked into
standard transforms and skinning; playback requires no Python handlers.

From the repository root, rebuild with Blender 4.3.2:

```sh
blender --background --factory-startup -t 4 \
  --python assets/brand/penguini/3d/source/animate_penguini.py
```

This overwrites `source/penguini-excited.blend` and `penguini-excited.glb`.
Direct edits to the animated scene should be saved separately before rebuilding.
The reference JSON is a fixed snapshot; updates to the 2D registry do not
automatically change this animation.

## Web use and validation

Load `penguini-excited.glb` in a glTF viewer and play **Excited - Jump and wave**.
It is a 3D animation prototype; its mesh density has not been optimized for mobile
production use. Reserve space for the hop above the standing model, provide pause
controls for repeating playback, and respect reduced-motion preferences.
App placement and the web rendering component are separate integration work.

Validation of the approved asset covered Blender loading, a single 6.6-second
GLB clip, connected limb geometry, matching neutral endpoints, and floor contact
through the sampled hops and landings. The browser preview was checked from
front, side, and rear views, including playback controls and reduced motion.
The recorded first and final resting frames matched pixel for pixel.

This model was built procedurally in Blender from the approved 2D mascot design.
The supplied Blender scenes and GLB are retained byte for byte from the approved
experiment. Review GIFs, videos, screenshots, archives, and the temporary viewer
are kept outside the repository.
