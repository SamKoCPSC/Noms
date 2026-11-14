# About tour model assignments

The story order, wording, twelve independent arrivals and camera route are unchanged.
`src/pages/about.rs::MODEL_FOR_STOP` selects the shared asset for each card.

| Stop | Category | Card | Model |
| --- | --- | --- | --- |
| 1 | About Noms | The Ultimate Recipe Manager | Kitchen |
| 2 | Why was Noms created? | A side project that grew in to something larger. | UBC-inspired university library (revision 4) |
| 3 | About the creator | Hi, I'm Sam! | Hobby bedroom |
| 4 | About the creator | The inconveniences of a cooking hobby. | European arcade supermarket with Asian pantry |
| 5 | About the creator | Building my own solution. | Personal office |
| 6 | About the creator | A project to support my job search. | Corporate workspace and pantry |
| 7 | About the creator | The same fundamental idea. | Brick-loft U-shaped kitchen (revision 5) |
| 8 | The Noms Philosophy | Just a simple and easy-to-use app. | Modern Baking Lab (revision 3) |
| 9 | The Noms Philosophy | Free and accessible. | Corner farmers’ market (revision 9) |
| 10 | The Noms Philosophy | Without the disruptions. | Potluck room |
| 11 | The Noms Philosophy | Respecting your privacy. | Potluck room |
| 12 | The Noms Philosophy | Building a community. | Garden cookout |

## Source revisions

- The farmers’ market uses approved revision 9: four enlarged carts, two per
  corner edge, with bakery, dairy, pantry and prepared foods, plus four picnic
  tables. Its eight assemblies use the shared matte finish, dynamic shadows and
  three-second entrance. Only the paving is treated as architecture; carts enter
  intact from the open front/right sides. The web derivative fits the existing
  geometry budget; the editable master is unchanged.

- The Modern Baking Lab uses approved revision 3: expanded retail displays, dough
  sheeter, two refrigerators, and the workbench shifted 0.85 m right. Its 24
  assemblies use the shared matte lighting, dynamic shadows and three-second
  entrance through the open front/right sides. The full master is retained; the
  web derivative uses bounded geometry reduction for the existing renderer budget.

- The brick-loft kitchen uses approved revision 5: no pendant, cloth or upright
  recipe display; the repositioned faucet clears the window sill. Its 17 assemblies
  and all 332,244 master triangles are retained in the web derivative.
  It uses the shared matte lighting, dynamic shadows and three-second entrance,
  entering through the open front/left sides of its right-wall layout.

- The library uses approved revision 4 in `assets/models/university-library/`,
  reconstructed from the supplied UBC hall photos: blue window panels, portraits,
  suspended glass, a 360-degree stair to an open second floor, four study laptops,
  five open computer-science textbooks and window-facing seating. Its 13 assemblies
  use the shared matte finish, dynamic shadows and three-second entrance from the
  open front/left sides. The master and rebuild sources are retained; a separate
  web derivative stays below the renderer's 400,000-triangle limit.
- The supermarket is the approved concept-03
  `assets/models/european-arcade-supermarket/models/european-arcade-supermarket.blend`.
- The corporate office includes the three humorous posters, personalized desks,
  different code screens, and the corrected cooler/recycling placement.

Web derivatives, budgets and reproduction instructions live in
[`assets/about/README.md`](../../../assets/about/README.md). The Blender masters
remain intact. Repeated cards share geometry while retaining independent entry
animation and shadow state.

## Tentative future replacements

The brick-loft kitchen replaces the earlier tentative coffee-shop assignment.
The coffee-shop asset remains available for another future card.

The two remaining philosophy cards still share the potluck room. These are suggestions,
not approved assignments:

| Card | Suggested next model |
| --- | --- |
| Without the disruptions. | Cozy campsite (existing asset) |
| Respecting your privacy. | New secluded reading nook |
