# Model collections

Editable 3D scene collections for future Noms designs. Collections include Blender masters, standard GLB exports, rendered previews and
rebuild sources. Multi-scene collections also include an offline gallery.

| Collection | Scenes | Preview |
| --- | --- | --- |
| [L-shaped Social Kitchen](monochrome-social-kitchen/README.md) | Revision 5: smaller oval roast facing right, a fuller potato tray and individual soup bowls; review asset for Without the disruptions | [Preview](monochrome-social-kitchen/previews/monochrome-social-kitchen.png) |
| [Riverside Cabin Campsite](riverside-cabin-campsite/README.md) | Reconstructed revision 4: two-storey cabin, river, folded pines and camping food; review asset for Respecting your privacy | [Preview](riverside-cabin-campsite/previews/riverside-cabin-campsite.png) |
| [University Reading Room](university-library/README.md) | Revision 4: UBC-inspired hall with populated study tables, blue window panels and a full-turn spiral stair; integrated in the origin-story card | [Preview](university-library/previews/university-library.png) |
| [Modern Baking Lab](modern-baking-lab/README.md) | Revision 3: bakery storefront, dough sheeter, refrigeration and right-shifted workbench | [Preview](modern-baking-lab/previews/modern-baking-lab.png) |
| [Brick-loft U-shaped Kitchen](brick-loft-kitchen/README.md) | Revision 5: open-front dining peninsula, coffee appliances, shared food and corrected faucet | [Preview](brick-loft-kitchen/previews/brick-loft-kitchen.png) |
| [Shared Corporate Workspace & Pantry](corporate-workspace-pantry/README.md) | Concept-05 corporate office, three workstations, shared kitchenette and lunch table | [Preview](corporate-workspace-pantry/previews/corporate-workspace-pantry.png) |
| [Modern European Arcade Supermarket](european-arcade-supermarket/README.md) | Larger concept-03 grocery revision with European deli and Asian pantry | [Preview](european-arcade-supermarket/previews/european-arcade-supermarket.png) |
| [Five New Places](community-spaces/README.md) | Specialty coffee shop, farmers’ market, artisan bakery, night market, recipe library | [Contact sheet](community-spaces/previews/collection.jpg) · [Offline gallery](community-spaces/gallery.html) |
| [Camp, Gather, Stock Up](campsite-market-grocery/README.md) | Cozy campsite, night market courtyard, neighborhood grocery store | [Contact sheet](campsite-market-grocery/previews/collection.jpg) · [Offline gallery](campsite-market-grocery/gallery.html) |

Both night markets are retained: Five New Places has three food kiosks and a
communal dining table; Camp, Gather, Stock Up has the later L-shaped lantern
courtyard with café tables and a separate milk-tea cart.

The production About tour assets and their loader contract are documented in
[`assets/about/README.md`](../about/README.md). These collections use standard
glTF review exports; they need an optimization and conversion pass before use
with the tour's bounded Rust loader.

The corporate workspace also has a production derivative in `assets/about/`,
used by the About tour’s “A project to support my job search.” card.

The UBC-inspired university library, European arcade supermarket, brick-loft kitchen,
Modern Baking Lab and revision 9 corner farmers’ market also have production
derivatives. See the [About tour assignment table](../../docs/design/about/model-assignments.md)
for the current cards and pending revisions.
