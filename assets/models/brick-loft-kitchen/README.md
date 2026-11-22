# Brick Loft U-shaped Kitchen

Review model based on approved kitchen concept 04 (Industrial Peninsula).

- Back run: refrigerator, gas range, extractor, prep counter and shelves.
- Right return: inset sink under the factory-style window.
- Open front edge: connected oak dining peninsula with three stools.
- Open left entrance; countertop forms a U with room to cook inside it.
- Matte charcoal framed cabinetry, individual warm brickwork, concrete floor,
  oak fronts, brass pulls, copper cookware, meal preparation and a handwritten recipe notebook.
- Revision 2: removed the ceiling pendant; added tomato pasta, salmon with
  asparagus and lemon, garlic bread, and a roast-vegetable sharing platter.

## Files

- `models/brick-loft-kitchen.blend`: editable Blender master with camera and lighting.
- `models/brick-loft-kitchen.glb`: standard portable geometry and PBR materials.
- `models/brick-loft-kitchen-assemblies.json`: seventeen separately named assembly groups.
- `previews/brick-loft-kitchen.png`: actual Blender render.
- `previews/brick-loft-layout.png`: higher-angle layout render.
- `previews/approved-concepts.png`: original five-concept design sheet (concept 04).
- `source/`: self-contained procedural rebuild source and shared Noms primitives.

Rebuild with Blender 4.3+: `blender -b --threads 6 --python source/build_kitchen.py`.

The master remains a static grouped model. The about tour uses a separate web
derivative with seventeen animated assemblies for “The same fundamental idea.”
See [web export instructions](../../about/README.md#brick-loft-kitchen). Lighting
and shadows are dynamic in Blender; no object-placement shadows are baked onto
walls or floor. The GLB uses portable base materials; Blender's subtle procedural
wood and brick detail is retained in the master and may need baking for web parity.

Revision 3 adds a home espresso machine, bean grinder, two-slot toaster and electric kettle, with wall outlets and tucked cables. The cutting board moves to the window counter to keep appliance footprints clear.

Revision 4 removes the upright recipe tablet and stand behind the salad. The tea towel is rebuilt to drape around the front edge of the countertop, with its evaluated underside checked for clearance; it is moved left to avoid the plate and cutlery.

Revision 5 removes the tea towel entirely. The faucet and mixer are deck-mounted at the short end of the sink, with the spout over the basin and more than 3 cm between their evaluated geometry and the window sill. The cloth is absent from the exported GLB.
