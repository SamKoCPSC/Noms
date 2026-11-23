# Model collections

Editable 3D scene collections for future Noms designs. Collections include Blender masters, standard GLB exports, rendered previews and
rebuild sources. Multi-scene collections also include an offline gallery.

| Collection | Scenes | Preview |
| --- | --- | --- |
| [Village outdoor walking study](terraced-village/README.md) | Twelve preserved scenes in an expanded landscape with a ground-following camera tour | [Overview](terraced-village/previews/overview.png) |
| [About-tour baked lighting](about-baked-tour/README.md) | Static Three.js derivatives for all twelve active tour scenes, with source references and reproducible lighting bakes | [Provenance and review](about-baked-tour/README.md) |
| [Terracotta Recipe Kitchen](terracotta-recipe-kitchen/README.md) | Original revision 13 master for the first About-tour scene, with packed textures and original lighting | [Original review](https://chatgpt.com/space/page_60c2cf7e836c8191a4c07002a7fdfc0c) |
| [Home Office](home-office/README.md) | Original revision 10 master, with packed artwork and original lighting | [Original review](https://chatgpt.com/space/page_65be16eb77f08191a6d3faeb22138385) |
| [Corner Cocoon Bedroom](corner-cocoon-bedroom/README.md) | Original revision 18 master, with keyboard desk, lounge and badminton corner | [Original review](https://chatgpt.com/space/page_dbc8487deee88191aa97de7c374084b6) |
| [Garden Cookout](garden-cookout/README.md) | Original revision 6 master, with shared tables, sports equipment and lawn chairs | [Original review](https://chatgpt.com/space/page_7c6bf3f7c53081919c1588f851054446) |
| [About-tour penguin plushes](about-penguin-plushes/README.md) | Fifteen companions using the approved Chef Penguini model across all twelve active tour models | [All scenes](about-penguin-plushes/previews/collection.jpg) · [Close-ups](about-penguin-plushes/previews/plush-details.jpg) |
| [L-shaped Social Kitchen](monochrome-social-kitchen/README.md) | Revision 8: matching oval beef servings, individual soups, island steak-sauce bowl and stovetop sauce pot; integrated in Without the disruptions | [Preview](monochrome-social-kitchen/previews/monochrome-social-kitchen.png) |
| [Riverside Cabin Campsite](riverside-cabin-campsite/README.md) | Reconstructed revision 4: two-storey cabin, river, folded pines and camping food; integrated in Respecting your privacy | [Preview](riverside-cabin-campsite/previews/riverside-cabin-campsite.png) |
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
[`assets/about/README.md`](../about/README.md). The collections retain standard glTF review exports. The tour now uses separate
baked Three.js derivatives with Rust camera control; the legacy bounded Rust
loader remains available as a fallback.

The corporate workspace also has a production derivative in `assets/about/`,
used by the About tour’s “A project to support my job search.” card.

The UBC-inspired university library, European arcade supermarket, brick-loft kitchen,
Modern Baking Lab and revision 9 corner farmers’ market also have production
derivatives. See the [About tour assignment table](../../docs/design/about/model-assignments.md)
for the current cards and pending revisions.
