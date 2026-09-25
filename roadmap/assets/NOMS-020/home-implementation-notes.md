# Home implementation refinement

The accepted composition remains [Cozy Home v2](public-home-cozy-v2.png). This refinement follows its illustrated hero, three compact benefit cards, and recipe-version notes while retaining the application's Fredoka/Nunito typography, shared palette, button/input styles, and elevation tokens.

The hero is decorative, not the canonical S1 app logo. Its cover label deliberately contains no generated typography. Navigation, search, authenticated creation destinations, and the longer feature explanations retain their existing behavior.

## Hero artwork

- Production asset: `assets/home-kitchen-illustration.png` (1536 × 1024 RGBA PNG).
- Created with the built-in image-generation tool, using `public-home-cozy-v2.png` as a style/composition reference.
- No third-party image was introduced.
- The generated alpha channel is preserved to blend with light and dark surfaces.

Generation prompt:

> Create a production website illustration asset, NOT a webpage or mockup. Reference image is style/composition reference only: recreate ONLY its beautiful right-hand hero still life as a standalone TRANSPARENT BACKGROUND PNG with true alpha. An olive green closed recipe notebook tilted slightly counterclockwise with cream smooth spine, cream pages, coral elastic vertical strap and coral bookmark; blank small warm ivory cover label with a tiny botanical sprig (NO TEXT). Beside it a warm ivory ceramic bowl with delicate olive leaf decorations filled with golden soup and herb garnish, a wooden spoon in foreground and a loosely folded sage gingham cloth beneath bowl. A few elegant herb sprigs around base. Cozy sophisticated hand-painted gouache illustration, smooth broad painted planes, restrained paper grain on objects, clear warm upper-left lighting, beautifully dimensional bevels, gentle soft contact shadows, warm cream olive and muted coral palette. Carefully drawn organic curves and unified perspective, charming but not childish or plastic. Compact balanced landscape arrangement, book on left/taller, bowl on right/lower. All objects fully contained with 5% transparent margin, no hard rectangular backdrop, no UI, no lettering, no slogans, no page text, no frames. Transparent background is essential so it blends with existing warm cream webpage. High quality finished illustration suitable for 600px wide hero.
