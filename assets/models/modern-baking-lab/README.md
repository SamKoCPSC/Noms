# Noms — Modern Baking Lab

Original Blender model based on selected artisan-bakery concept **10**.
A clean contemporary bakery with warm oak joinery, matte cream walls, stainless-steel
work surfaces, tiled flooring and detailed bread and pastry displays.

## Revision 3

Shifted the central workbench 0.85 m to the right, including the mixer, dough,
roll tray, tools and under-bench storage, to make it more visible beyond the
front counter. Rear-counter equipment stays in place. The counter service aisle
is preserved, with over 0.8 m between the workbench and cooling rack.

## Revision 2

Expanded the retail frontage and platform, added a reversible dough sheeter with
conveyors, safety guards and thickness wheel; a stainless ingredient refrigerator;
a glass cake/drinks refrigerator; raised wall bread shelves and bakery signage;
a checkout with till, card reader and takeaway bags; a refrigerated tart/cake
counter; and freestanding baguette baskets and a chalkboard menu. The cooling
rack moves forward to make room for refrigeration. The new customer counter
leaves a one-meter service aisle behind it.

## Included

- `models/modern-baking-lab.blend` — editable Blender 4.3 master, with camera, studio lights and materials.
- `models/modern-baking-lab.glb` — portable standard glTF model; no external textures required.
- `models/modern-baking-lab-assemblies.json` — 24 separately named assembly groups and their bounds.
- `previews/modern-baking-lab.png` — actual Blender hero render.
- `previews/modern-baking-lab-layout.png` — higher-angle layout render.
- `previews/approved-concepts.png` — concept sheet; design 10 is the reference.
- `source/` — complete procedural rebuild scripts and the existing Noms croissant geometry.
- `validation.json` — exported geometry and assembly checks.

The scene includes a double deck oven with visible baking bread, a glass proofing
cabinet, six-tier wheeled cooling rack, ingredient jars, a washing/prep counter,
a central island with stand mixer, dough and shaped rolls, and a sloped glass case
with sourdough, baguettes, croissants and cinnamon buns. It uses separate assembly
roots for later animation; the review master itself is static. A separate web derivative is integrated into
“Just a simple and easy-to-use app” in the about tour, with shared lighting,
shadows and entry animation. See [web export instructions](../../about/README.md#modern-baking-lab).

## Rebuild

```sh
blender -b --threads 6 --python source/build_bakery.py
blender -b --threads 6 --python source/render_detail.py
python source/validate_model.py
blender -b --python source/check_import.py
# Optional denoise: requires Pillow, numpy and pyoidn.
python source/denoise_previews.py modern-baking-lab modern-baking-lab-layout
```

The Blender master retains procedural wood grain and physically lit glass.
The GLB contains the evaluated geometry and portable base PBR materials;
subtle procedural wood detail is not baked. The host viewer supplies lighting.
There are no baked object-placement shadows. Studio geometry is excluded from
GLB export. Open the `.blend` and use the supplied camera to reproduce the preview.
