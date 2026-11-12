# Shared Corporate Workspace & Pantry

Blender design-review model based on selected **concept 05** for the About tour's
“A project to support my job search” section. The scene combines three corporate
workstations with a shared kitchenette and six-seat lunch table in the warm,
matte Noms miniature style.

The desks include code screens, recessed keyboards, contoured mice, cable trays,
compact PCs, coffee cups and plants. Each mesh task chair has a five-star caster
base, armrests and attached lumbar support. The pantry contains separate cabinet
doors, a working sink opening, faucet, microwave, refrigerator, coffee machine,
mugs, supplies and bottled-water cooler. Lunch includes eight layered sandwich
wedges, a fruit bowl, water pitcher, glasses, napkins and cutlery.

The water cooler and recycling bin sit together to the left of the kitchenette,
leaving the refrigerator approach clear. The three monitors show distinct source
code: Rust recipe filtering, TypeScript ingredient scaling, and a Rust API test.
Each workstation has a different owner’s setup: a sage mat, travel tumbler,
phone and plant; a cream mat, open notebook, pencils, sticky notes and coffee;
and a slate mat, headphones, pocket journal, coffee and chocolate-chip cookies.
Keyboard accent colors vary with the desk accessories.
Above the desks, two oak-framed posters replace the acoustic panels:
“LET'S TAKE THIS OFFLINE.” shows a figure stepping off a line, and
“LET'S CIRCLE BACK ON THAT.” shows a walking figure inside a circular arrow.
Above the water cooler, “DO WE HAVE ENOUGH BANDWIDTH?” replaces the bulletin
board with five coworkers crowded onto a narrow band. All poster lettering
and illustrations are editable geometry and export without external textures.

- [Rendered preview](previews/corporate-workspace-pantry.png)
- Poster close-ups: [Offline](previews/offline-poster.png),
  [Circle back](previews/circle-back-poster.png), [Bandwidth](previews/bandwidth-poster.png)
- [Editable Blender master](models/corporate-workspace-pantry.blend)
- [Standard review GLB](models/corporate-workspace-pantry.glb)
- [Assembly metadata](models/corporate-workspace-pantry-assemblies.json)
- [Selected concept sheet — 05](previews/approved-concepts.png)
- [Validation](validation.json)

Fifteen named assembly roots separate the architecture, workstation equipment,
chairs, kitchenette, appliances, lunch table and food for later animation. This
is a static review master. Its grouped web derivative is integrated into the
About tour's job-search card; see [the web assets](../../about/README.md).
The review GLB excludes
studio lights and cameras; its viewer supplies lighting. The native Blender
master retains editable type, curves, modifiers, procedural wood and lighting.
There are no external textures or baked placement shadows.

## Rebuild

From the repository root, with Blender 4.3.2:

```sh
blender -b --threads 4 --python assets/models/corporate-workspace-pantry/source/build_office.py
blender -b assets/models/corporate-workspace-pantry/models/corporate-workspace-pantry.blend --python assets/models/corporate-workspace-pantry/source/validate_model.py
python assets/models/corporate-workspace-pantry/source/denoise_previews.py corporate-workspace-pantry
```

Use `-- --proof` on the build command for a 900 × 800px / 16-sample proof.
The final render is 1800 × 1600px / 64 samples. Denoising requires `pyoidn`, NumPy
and Pillow. Shared palette and furniture helpers are loaded from
`../campsite-market-grocery/source/`; retain that directory when rebuilding.

Validation checks assembly membership, finite geometry, scene bounds, sink
clearance, workstation/pantry circulation, chair counts and the GLB structure.
