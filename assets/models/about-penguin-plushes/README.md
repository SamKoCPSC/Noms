# Chef Penguini companions for the About tour

Fifteen small Penguini figures across the twelve active About scenes: one per
indoor scene and two each in the farmers’ market, campsite and garden. Existing
bedroom penguins are retained. This revision replaces the earlier scarf-plush
interpretation with the [approved Chef Penguini model](../../brand/penguini/3d/README.md).

The derivative keeps Penguini’s heart-shaped cream face, charcoal body and joined
flippers, ivory chef’s hat, golden feet and beak, and terracotta apron with pocket
and ties. It is extracted from the actual approved `base.blend`, not rebuilt from
an approximation. Its resting pose uses the tour’s existing matte lighting,
dynamic shadows and three-second entrance. The mascot’s jump-and-wave clip is
not included in these static scene accessories.

[All scenes](previews/collection.jpg) · [Placement close-ups](previews/plush-details.jpg) ·
[Editable derivative](models/penguini-plush.blend) · [GLB derivative](models/penguini-plush.glb)

Each figure adds 2,841 triangles and one draw call without image textures. The
garden remains within the 400,000-triangle limit at 399,867 triangles; further
additions there require optimization.

## Placements

The home-office Penguini stands on the desk mat between the keyboard and monitors,
well clear of the window sill. The bedroom and checkout positions are adjusted
for the approved model’s wider flippers and different feet.

| Scene | New Penguinis | Placement | Delivered triangles |
| --- | ---: | --- | ---: |
| kitchen | 1 | Above the refrigerator | 190,790 |
| library | 1 | Front window lounge chair | 377,149 |
| bedroom | 1 | Lavender lounge chair; existing penguins retained | 344,848 |
| supermarket | 1 | Checkout bagging counter | 387,643 |
| office | 1 | Desk between keyboard and monitors | 243,532 |
| corporate-office | 1 | Front workstation, clear of the mouse and monitor | 267,670 |
| brick-kitchen | 1 | Left dining stool | 335,085 |
| bakery | 1 | Bakery checkout mascot | 380,634 |
| farmers-market | 2 | Front-left picnic bench; Front-right picnic bench | 360,787 |
| social-kitchen | 1 | Window counter beside the herb plant | 365,415 |
| riverside-campsite | 2 | Wooden porch lounge; Picnic bench beside the cabin | 161,725 |
| garden | 2 | Front picnic bench; Beer-table picnic bench | 399,867 |

## Source and reproducibility

`library-metadata.json` records the approved source SHA, per-part simplification
counts and foot-contact probes. The source model and animation remain untouched.
Small decorative piping is omitted in this scale derivative. Thin clothing and
face patches use single surfaces with a tiny outward offset to prevent the
simplified front and back layers from intersecting. All major character parts
come from the approved geometry and keep their source colors.

```sh
blender -b --threads 2 --python assets/models/about-penguin-plushes/source/build_plush_library.py
```

`placements.json` records the source variant, furniture support, size and glTF
Y-up transform for each figure. The native helper converts these transforms to
Blender Z-up and creates a separate editable copy of an available scene master:

```sh
blender -b assets/models/monochrome-social-kitchen/models/monochrome-social-kitchen.blend \
  --python assets/models/about-penguin-plushes/source/apply_to_master.py -- \
  social-kitchen /tmp/social-kitchen-with-penguini.blend
```

The production pass appends accessories to the recorded clean optimized GLBs.
Original buffers, materials, UVs, accessors, primitives and node transforms are
preserved. The original scene masters remain unchanged; the derivative and
placement manifest are the reproducible source for this accessory layer.

```sh
mkdir -p /tmp/about-plush-base
git archive da47d8b908ab566ddf96cd2b66c8213621a6ef95 assets/about | tar -x -C /tmp/about-plush-base
python tools/about/add_penguin_plushes.py --base-dir /tmp/about-plush-base/assets/about
python tools/about/verify_penguin_plushes.py --base-dir /tmp/about-plush-base/assets/about
python tools/about/check_plush_clearance.py
```

Always use clean baseline inputs. Changed input hashes are rejected so placements
are reviewed when the furniture changes. `integration-audit.json` records additions
and preserved binary hashes. Verification checks source provenance, budget limits
and actual support triangles under both feet. The separate triangle-intersection
check verifies clearance from the surrounding geometry above the feet, including
sills and chair backs that a foot-height check would miss. Contact within 8 mm of
the base is allowed for curved cushions and seats.

## Review renders and reading-mode images

The previews render the actual composed production GLBs, restoring external
color artwork for the older rooms. They are Blender renders, not browser
screenshots. App rendering uses the shared runtime shader and room lighting.

```sh
blender -b --threads 2 --python assets/models/about-penguin-plushes/source/render_review.py -- office /tmp/penguini-review
```

Repeat for all model keys in `placements.json`; `library-gallery` renders the
approved Penguini derivative alone. Full scenes are 1080 × 945 and close-ups are
480 × 480. After all renders finish:

```sh
python assets/models/about-penguin-plushes/source/publish_previews.py /tmp/penguini-review
```

This publishes the review sheets and matching `assets/about/sams-*.webp` posters
used by the reading and graphics-fallback modes.
