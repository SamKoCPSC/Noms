# About tour model assignments

The story order, wording, twelve independent arrivals and camera route are unchanged.
`src/pages/about.rs::MODEL_FOR_STOP` selects the shared asset for each card.

| Stop | Category | Card | Model |
| --- | --- | --- | --- |
| 1 | About Noms | The Ultimate Recipe Manager | Kitchen |
| 2 | Why was Noms created? | A side project that grew in to something larger. | Recipe library |
| 3 | About the creator | Hi, I'm Sam! | Hobby bedroom |
| 4 | About the creator | The inconveniences of a cooking hobby. | European arcade supermarket with Asian pantry |
| 5 | About the creator | Building my own solution. | Personal office |
| 6 | About the creator | A project to support my job search. | Corporate workspace and pantry |
| 7 | About the creator | The same fundamental idea. | Hobby bedroom |
| 8 | The Noms Philosophy | Just a simple and easy-to-use app. | Potluck room |
| 9 | The Noms Philosophy | Free and accessible. | Potluck room |
| 10 | The Noms Philosophy | Without the disruptions. | Potluck room |
| 11 | The Noms Philosophy | Respecting your privacy. | Potluck room |
| 12 | The Noms Philosophy | Building a community. | Garden cookout |

## Source revisions

- The library is the existing `assets/models/community-spaces/models/recipe-library.blend`.
  The later UBC-inspired library revision discussed in chat was not found in the
  available repository or workspace. That revision remains pending; this integration
  does not claim to include its computer-science books or window-side seating.
- The supermarket is the approved concept-03
  `assets/models/european-arcade-supermarket/models/european-arcade-supermarket.blend`.
- The corporate office includes the three humorous posters, personalized desks,
  different code screens, and the corrected cooler/recycling placement.

Web derivatives, budgets and reproduction instructions live in
[`assets/about/README.md`](../../../assets/about/README.md). The Blender masters
remain intact. Repeated cards share geometry while retaining independent entry
animation and shadow state.

## Tentative future replacements

A specialty coffee shop was suggested for “The same fundamental idea.” It remains
unassigned. Further philosophy-card replacements are also deferred until their
models are selected; no additional story-copy changes are implied by this plan.
