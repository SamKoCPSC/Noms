# Modern European Arcade Supermarket

A larger revision of the neighborhood grocery, following selected concept **03**.
Cream plaster arches frame the deli, with an Asian pantry and dumpling freezer
along the left wall. Three double-sided grocery aisles connect the produce
island, dairy and beverage refrigerators, two checkouts, and shopping carts.

The original grocer's matte materials, product designs, checkout and wire cart
geometry are reused. New deli fittings include a glazed service counter,
cheese wheels and wedges, sliced charcuterie, olive bowls, hanging cured meats,
a preparation counter and bread baskets. Seventeen named assembly roots allow
future entrance animation. The scene is static for design review.

- [Reference concept sheet — selected design 03](previews/approved-concepts.png)
- [Rendered preview](previews/european-arcade-supermarket.png)
- [Editable Blender master](models/european-arcade-supermarket.blend)
- [Standard review GLB](models/european-arcade-supermarket.glb)
- [Assembly metadata](models/european-arcade-supermarket-assemblies.json)
- [Validation report](validation.json)

This is a review asset; it has not been connected to the About tour. The GLB
uses standard PBR materials and geometry and requires lighting from its viewer.
The Blender master retains editable text, curves, modifiers, procedural wood,
and studio lighting. All artwork is geometry; there are no external textures.

## Rebuild

Run from the repository root with Blender 4.3.2:

```sh
blender -b --threads 4 --python assets/models/european-arcade-supermarket/source/build_supermarket.py
blender -b assets/models/european-arcade-supermarket/models/european-arcade-supermarket.blend --python assets/models/european-arcade-supermarket/source/validate_model.py
python assets/models/european-arcade-supermarket/source/denoise_previews.py european-arcade-supermarket
```

The denoising step requires `pyoidn`, NumPy and Pillow. Use `-- --proof` on the
build command for a quicker small render; rebuild without it for final quality.
Shared helpers and original grocery source are loaded from
`../campsite-market-grocery/source/`; retain the repository structure.
