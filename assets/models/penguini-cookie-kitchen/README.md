# Penguini’s cookie kitchen

Penguini drops a pat of butter into a saucepan and checks his recipe book while
the butter browns. The mixing bowl sits on a sage kitchen scale with a blank
display facing Penguini. The bowl starts empty. Browned butter spreads across
the bottom, and the level rises with each addition. He adds brown sugar, cracks
two eggs on the rim, and adds vanilla and baking soda. Next he fetches a small
jar of espresso powder from beside the coffee. He unscrews the lid, holds it
in his free flipper while shaking powder into the bowl, closes the jar and
returns it to the back counter. He waddles with alternating footfalls on each
trip and holds his head level while he twists the cap. He stirs the wet
ingredients before fetching the flour and chocolate chips, then mixes the
dough a second time. The chips turn with the spoon and remain visible at the
surface as he scoops. Each portion he rolls leaves less dough in the bowl. Between
stirs, the wooden spoon rests behind the baking tray. Steam rises from the
soup and coffee on the back counter.

The spare sugar ramekin and salt shaker are saved without animation in the Blender collection
`Recipe ingredients | unanimated props`. Link this collection back into the
kitchen scene to reuse the models when authoring ingredient additions.

The Blender 4.3.2 timeline runs for 142 seconds at 30 fps. Penguini places two
cookies at the back of the tray, then leans forward to set the third at the front
center with his left flipper. They bake on that same sheet of parchment. Penguini eats one cookie in five
bites, then does a short dance while the other two stay on the tray.

For a straight ingredient reach in Blender, move `Mixing | gentle waist lean` and
swing `Flipper.L/R` at the shoulder. Match the shoulder helper to that swing,
keep the flipper length fixed, and leave `Foreflipper.L/R` at its rest rotation.
For the espresso jar, both elbows swing outward to clear the apron straps
while the palms meet the jar and cap. Rotate `Head • rotate here` at the collar
to aim his gaze.
For a deeper lean, match `Body and clothing` to the `Torso` bone. The skin
and apron blend this motion into the waist while the feet stay planted.
The kitchen placement object moves the hips. The two integrated ankle objects
carry the footfalls. On the turns between the pantry and bowl, the hips shift
gently toward the planted foot. After he sets the book down, Penguini crosses
to the saucepan in short alternating steps. His shoulders rock over each
planted foot, then settle as his flipper meets the handle. As the flour sack
or chocolate chip crock tips, the torso leans 10° and the free flipper rises
8° from its carrying pose, matching the brown-sugar pour.

| Time | Action |
| --- | --- |
| 0:00–0:06 | Welcome jump |
| 0:06–0:13 | Keep the torso upright while lifting the butter pat and dropping it into the saucepan |
| 0:13–0:19 | Read the recipe and return the book to its stand |
| 0:19–0:27 | Pour the browned butter into the bowl and return the saucepan |
| 0:27–0:32 | Add the brown sugar |
| 0:32–0:39 | Tap and crack two eggs on the rim, one at a time |
| 0:39–0:49 | Pour vanilla and shake baking soda into the bowl |
| 0:49–1:02 | Fetch the espresso powder, unscrew the lid, shake some in, close the jar and put it back |
| 1:02–1:09 | Pick up the spoon, stir the wet ingredients and set it down |
| 1:09–1:24 | Fetch the flour and chocolate chips from the back counter and add each to the bowl |
| 1:24–1:30 | Stir the dough and return the spoon behind the tray |
| 1:30–1:42 | Shape three cookies and place them on the tray |
| 1:42–1:56 | Load the oven and dance while the cookies bake |
| 1:56–2:06 | Put on both mitts, remove the tray and hang up the mitts |
| 2:06–2:18 | Cool the cookie, take five bites and step back from the tray |
| 2:18–2:22 | Celebrate with a happy dance and settle for the loop |

The Blender master includes a packed bell sound at the timer marker. The web
scene uses the moving bell and Penguini’s turn toward the oven as its silent cue.

`assets/landing/kitchen/penguini-kitchen.glb` is the browser derivative. Stationary
surfaces use two embedded 2048px, 128-sample lighting atlases. The utensils,
dough, oven door, mitts and cookies keep their animation. Steam stays
translucent. The browser uses lighter character geometry. The lighting manifest
records the editable master's SHA-256 and bake settings.

The kitchen uses the Noms light-theme colors from `assets/main.css`. The
cabinetry, backsplash and ceramic accents use a more saturated terracotta and
sage. Warm ivory walls and countertops frame the charcoal stove. The lighting
manifest records each source token and the paint color used in the bake.

To rebuild, copy the master and the scripts in `source/` into an external
working directory (bakes generate large intermediates). Run there:

```sh
blender -b penguini-cookie-kitchen-baking.blend --python-exit-code 1 --python check_recipe.py
# Optional: refresh material colors if the app palette has changed.
blender -b -t 4 --python apply_noms_palette.py -- penguini-cookie-kitchen-baking.blend /path/to/Noms/assets/main.css
blender -b -t 6 --python bake_static.py
blender -b -t 4 --python build_preview_lod.py
```

Then from the repo root:

```sh
node tools/home/three/pack.mjs WORK/preview-lite.glb assets/landing/kitchen/penguini-kitchen.glb
```

Copy `WORK/lighting-bake.json` to `assets/landing/kitchen/lighting.json` after a
rebake. The source script records Three-coordinate softbox positions and Blender
rotations for the shared reflection environment. Packing preserves all vertex
and animation values; Meshopt may cyclically rotate triangle indices without
changing winding. The pack command verifies this after decoding the export.

The GLB contains a single clip. The web renderer loops it with a brief fade as
the tray and bowl reset. It handles pause/resume, reduced motion and offscreen
suspension.
The poster is an intentionally static fallback, not a validation screenshot.
