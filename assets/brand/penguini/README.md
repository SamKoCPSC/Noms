# Chef Penguini

Approved penguin mascot assets, selected and refined on 2026-11-03–04. Chef Penguini
wears a cream chef hat and terracotta apron, without a neckerchief. These assets
are ready for integration; no app page currently displays the mascot.

The approved [3D model and excited jump-and-wave](3d/README.md) are also available
as a GLB and editable Blender scenes. They include the connected limb revision
and a 6.6-second animation with two hops, a wave, and facial expressions.

## Approved animations

| Asset | Behavior | Duration |
| --- | --- | --- |
| [Idle](animations/idle.svg) | Gentle sway, breathing, and occasional blinks | 6-second repeating motion |
| [Jump and wave](animations/excited.svg) | Two excited hops, a wave, and a gradual flipper recovery | 6.6 seconds |
| [Shy hello](animations/shy-hello.svg) | A small wave with a warm smile and head tilt | 5.2 seconds |
| [Sleepy moment](animations/sleepy.svg) | Dozes, jolts awake with wide eyes and a flipper flinch, then settles | 6.4 seconds |
| [Happy dance](animations/happy-dance.svg) | A small side-to-side dance | 5.8 seconds |
| [Thinking](preview.html?animation=thinking) | A gentle elbow bend to the chin, head tilt, and thoughtful squint | 6.8 seconds |
| [Silly dance](preview.html?animation=silly-dance) | Alternating flipper swings, waddling steps, two hops, and a finishing shimmy | 7.2 seconds |
| [Sneeze](preview.html?animation=sneeze) | Two tiny inhalations, squeezed eyes, a quick sneeze, and a recovery blink | 5.2 seconds |
| [Yawn](preview.html?animation=yawn) | Drooping eyes, a wide yawn and slow stretch, then a relaxed exhale | 7.2 seconds |
| [Wiggle dance](preview.html?animation=wiggle-dance) | Planted feet, side-to-side body wiggles, alternating flippers, and a finishing shimmy | 6.4 seconds |
| [Head wiggle](animations/head-wiggle.svg) | Soft alternating head tilts, expressive blinks, a happy smile, and a gradual return to rest | 5.2 seconds |

[still.svg](still.svg) provides the neutral resting pose. Standalone activities
play once and finish at that pose; idle repeats. Every animated SVG respects
`prefers-reduced-motion`. All SVGs have a transparent background and the same
`1200 × 1390` viewBox, preserving size when switching between them.

Thinking, Silly dance, Sneeze, Yawn, and Wiggle dance use the inline player and
WebGL renderers; their individual previews are linked above. Thinking keeps
the original artwork solid over the torso, with a low reach, soft easing, and a 25% faster
lift and recovery than the preceding review draft. Silly dance moves both flippers
independently, with a continuous shoulder join that avoids a background notch
under the lowered left flipper. Sneeze, Yawn, and Wiggle dance reuse that shoulder
mesh; the same renderer also manages their closed-eye expressions and breath cue.
All join the random and ordered activity sets and return smoothly to idle.
The other activities retain their standalone SVGs.

Head wiggle also joins the random and ordered activity sets. Its individual
preview is available at `preview.html?animation=head-wiggle`, and its standalone
SVG plays without JavaScript or WebGL.

The tummy-pat, chef's-kiss, and hat-adjustment experiments were not
approved and are not included in the animation registry. Review videos,
screenshots, concept sheets, and download archives are also excluded.

## Preview

From the repository root:

```sh
python3 -m http.server 8080 --bind 127.0.0.1 --directory assets/brand/penguini
```

Open `http://127.0.0.1:8080/preview.html`. The preview uses local files and no
external services. Choose a single activity, random playback, or all activities
in order. Pause/resume and reduced-motion behavior are available in the preview.

## Web integration

For a standalone activity, use its animated SVG as an image. Reserve dimensions
in the `1200:1390` aspect ratio (for example `240 × 278`) and use `alt=""` when
decorative beside existing content. A meaningful standalone image needs an
appropriate accessible description. Provide a pause control for repeating motion;
an image-based implementation can swap to `still.svg`.

For the reusable player, insert `source/rig.svg` inline and load `clips.js`,
`player.js`, `thinking-rig.js`, and `dance-rig.js` locally, then initialize:

```js
const thinking = await PenguiniThinkingRig.create(svgElement);
const dance = await PenguiniDanceRig.create(svgElement);
const player = new PenguiniPlayer(svgElement, {
  idle: PenguiniClips.idle,
  animations: Object.values(PenguiniClips).filter(clip => clip.id !== "idle"),
  renderers: {thinking, dance},
  idleDurationMs: 4000,
  transitionMs: 240,
});
```

The player spends four seconds in idle, selects a registered activity at random,
lets it finish, and returns to idle. Brief 240ms blends join the phases. Repeats
are allowed. Each activity starts and ends in the resting pose, with a settled
tail before returning to idle. The jump flipper lowers gradually over 1.2 seconds.

Connect visible controls to `player.pause()` and `player.play()`. Call
`player.destroy()`, then `thinking.destroy()` and `dance.destroy()` on unmount.
Renderers are borrowed by the player and can be reused when replacing a player
on the same rig. Each renderer restores the original artwork when inactive.
The player pauses when hidden or offscreen, keeps manual pause state, and
displays a still pose for reduced-motion preferences.
Use one inline rig per player; give SVG definition IDs unique names when mounting
multiple inline mascots in one document. Import assets through the app's asset
pipeline when adding a Dioxus component; this folder is not globally loaded by
`Dioxus.toml`.

`PenguiniThinkingRig.create()` and `PenguiniDanceRig.create()` resolve after decoding
the embedded artwork and preparing their meshes. If WebGL is unavailable, omit
clips with a `renderer` field and use the existing SVG activities; the preview
demonstrates this fallback.
Registering a procedural activity without its renderer fails explicitly, so a gesture
cannot silently play without its articulated flipper. Meshes and clip masks are
created at runtime and removed on renderer disposal. The shared illustration
and the other animations' appearance are unchanged.

## Editing and extending

- `source/rig.svg` contains the shared illustration and separately animated parts.
- `source/clips.json` is the canonical registry of approved animation keyframes.
- `player.js` owns playback, selection, transitions, and lifecycle behavior.
- `thinking-rig.js` owns the articulated arm, squint, and approved movement timing.
- `dance-rig.js` deforms the left flipper using the `.left-flipper` control track
  and manages the sneeze/yawn expressions and breath cue.
- `clips.js`, `still.svg`, and `animations/*.svg` are generated assets.

After changing the rig or keyframes, regenerate and verify from the repo root:

```sh
python3 scripts/build-mascot-assets.py
python3 scripts/build-mascot-assets.py --check
```

The embedded illustration is deliberately shared across the generated files so
each SVG remains self-contained and works in an `img` without external resource
requests. Do not edit those generated copies by hand. The generator uses only
Python's standard library.

New activities can be added to the source registry, or registered at runtime with
`player.registerAnimation({ id, label, durationMs, tracks })`. Tracks target the
rig's existing selectors and contain Web Animations `transform`/`opacity`
keyframes. Use unique IDs; `idle` is reserved. Preserve matching resting endpoints
and leave time for recovery. Unspecified parts retain their resting pose.

Clips that need procedural drawing declare a `renderer` key. Supply that key in
the player's `renderers` map with `draw(state)` and `reset()` methods. These use
the same clock as the keyframes, including deterministic advancement, pauses,
phase changes, and reduced motion. The generator includes these clips in
`clips.js` but deliberately does not produce an incomplete animated SVG.

## Artwork provenance

Generated with the image-generation tool and refined with user feedback. The
selected concept was Chef Penguini: a rounded charcoal-and-cream penguin with a
floppy chef hat. Subsequent revisions added the terracotta apron, removed the
sage neckerchief, enlarged the open eyes, and separated the head/flipper for
animation. The rig embeds the transparent raster illustration and adds vector
facial features; these are not pure-vector illustrations. Preserve the warm
palette, soft outlines, aspect ratio, and alpha when preparing derivatives.

Validation covers the approved clips' resting endpoints, the sleepy jolt,
random/ordered selection, pause/resume, offscreen behavior, and reduced motion.
Procedural gestures are also checked against their approved review poses, with
browser checks for switching to other activities, expression restoration,
renderer disposal, and the WebGL fallback.
The gallery and standalone SVGs are the review surfaces; captured review media
remain outside the repository.
