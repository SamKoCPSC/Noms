# Landing-page imagery

The kitchen journal landing page implements the approved proposal 6 revision.
`table.webp` and `cooking.webp` are generated kitchen photography. The other
WebP files show the actual local Noms app, populated with synthetic recipes and
the example cook Robert McRobertson. The food photographs in those captures
were generated and uploaded through the recipe editor.

`calculator-checklist.webp` composes captures of the actual calculator and mobile
ingredient checklist: chili garlic oil noodles scaled from two to four servings,
with two ingredients checked. `cook-1.webp` through `cook-3.webp` are successive
steps in the real mobile cooking view for miso mushroom ramen. CSS animates these
three frames; a pause control and reduced-motion fallback are provided.

These are static illustrations, independent of a visitor's account or database.
They contain no real user data or authentication state. Local fonts and hashed
assets are served by Dioxus; no third-party image or animation service is used.
