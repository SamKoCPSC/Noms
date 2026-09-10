# NOMS-016 Clipboard Import Compatibility Spike

**Status:** In progress
**Started:** 2026-08-20
**Parent plan:** [NOMS-016 recipe import and export](../implementation-plans/NOMS-016-recipe-import-export.md)

## Decision this spike must enable

Determine the smallest deterministic clipboard-import pipeline that recovers useful recipe drafts from the primary provider set, and identify the providers or page shapes that require a source adapter, a different copy workflow, or an explicit unsupported result.

This spike tests what a user can paste into Noms. The presence of Schema.org data in the original page is useful supporting evidence, but it is not evidence that the structured data survives a normal browser copy operation.

## Evidence levels

- **Observed clipboard:** `text/html` and `text/plain` were captured from an actual browser copy operation and inspected.
- **Observed markup:** the public page or print view was inspected, but its clipboard representations were not captured.
- **Inferred:** behavior was inferred from rendered markup, structured data, or provider documentation and must be confirmed manually.

Only observed-clipboard evidence can close a browser/provider cell in the compatibility matrix. Every observation records the date, operating system, page mode, selected region, clipboard MIME types, access state, representative URL, and browser/version when the capture environment exposes them. A capture with an unknown engine or version is useful page-shape evidence but cannot close a named-browser matrix cell.

## Representative cases

For each primary provider, test the smallest available set that covers:

1. A simple recipe copied from the visible recipe card.
2. A recipe with ingredient or instruction sections.
3. A component or subordinate recipe, when the provider supports one.
4. A print-friendly view, when available without bypassing access controls.
5. A recipe with an absent, textual, range-based, or otherwise non-numeric time or yield.

The proposed initial browser matrix is current stable Chrome, Firefox, and Safari on a supported desktop operating system. Chrome is the proposed first release gate; Firefox and Safari may ship with documented plain-text fallback differences if they preserve all required recipe content. This release policy remains subject to product approval after the captures are compared.

## Capture record

Each clipboard sample records:

| Field | Required evidence |
|---|---|
| Provider and URL | Public provider name and representative URL |
| Access state | Public, consent wall, account required, subscription boundary, or unavailable |
| Browser context | Browser/version when exposed, OS, normal or print view, and selected region; explicitly record unavailable details |
| Clipboard types | Ordered MIME-type list from the paste event |
| HTML shape | Minimal structural summary: headings, lists, sections, links, and provider-added wrappers |
| Plain-text shape | Heading, blank-line, bullet, numbering, section, and attribution behavior |
| Recovered fields | Title, description, author, source URL, times, yield, ingredients, instructions, sections, images, and notes |
| Noise and ambiguity | Navigation, advertisements, ratings, reviews, duplicated units, scaling controls, or missing boundaries |
| Parser result | Generic HTML, generic text, source adapter, alternative copy workflow, or unsupported |
| Warnings | Information the review UI must show |
| Fixture | Minimal synthetic structure needed for a repository test |

Do not retain full third-party pages or recipes in the repository. Synthetic fixtures should use invented recipe content and reproduce only the structural features needed by the parser.

## Classification rules

- **Generic HTML:** preserves ordered ingredients and instructions, section boundaries, and the title without unrelated page content being mistaken for recipe data.
- **Generic text:** meets the same content requirements using recognizable headings, bullets, numbering, and blank lines.
- **Source adapter:** a stable provider marker and a small deterministic structural rule resolve a failure that the generic parser cannot resolve safely.
- **Alternative workflow:** normal page copy is unsuitable but a user-visible print or recipe-card view produces a deterministic payload.
- **Unsupported:** required content is unavailable, access is blocked, or extraction would require heuristic/NLP guessing.

A successful candidate may leave optional metadata unset, but it must preserve every visible ingredient and instruction in source order. Unparsed ingredient quantities remain intact as raw ingredient names. Ambiguous boundaries, omitted metadata, and discarded media must produce warnings rather than silent guesses.

## Primary provider matrix

| Provider | Markup evidence | Clipboard evidence | Provisional parser tier | Open verification |
|---|---|---|---|---|
| Allrecipes | Public rendered markup inspected; origin challenge prevented source confirmation | Whole-page, two normal-page selections, and print view captured in Codex in-app browser on Linux | Print workflow preferred; adapter for normal-page fallback | Chrome, Firefox, Safari confirmation |
| Food52 | Public rendered markup inspected; origin challenge prevented source confirmation | Whole-page normal copy captured; copy attempted while browser print UI was shown, with selection source uncertain | Normal-page workflow is the current candidate; generic parser plus narrow cleanup | Additional recipes, grouped ingredients/video boundary, focused selection, Chrome/Firefox/Safari confirmation, parser fixture |
| Food Network | Public rendered markup inspected; origin blocked direct source requests | Whole-page normal copy captured; copy attempted while browser print UI was shown, with selection source uncertain | Normal-page workflow is the current candidate; generic parser plus narrow cleanup | Additional recipes, suspicious durations, multi-paragraph steps, focused selection, Chrome/Firefox/Safari confirmation, parser fixture |
| NYT Cooking | Rendered markup and Recipe JSON-LD directly inspected | Whole-page normal view captured in Codex in-app browser on Linux; logged-out `Print Recipe` redirected to sign-in | Generic HTML candidate; adapter only if fixture-required | Subscriber print behavior, additional recipes, Chrome/Firefox/Safari confirmation, parser fixture |
| Serious Eats | Public rendered markup inspected; origin challenge prevented source confirmation | Whole-page normal and print-route views captured in Codex in-app browser on Linux | Print workflow provisionally preferred; generic parser plus boundary cleanup | Additional recipes, Chrome/Firefox/Safari confirmation, parser fixture |
| BBC Good Food | Rendered markup and Recipe JSON-LD directly inspected | Whole-page normal view captured in Codex in-app browser on Linux; no print-labelled control found | Generic parser plus narrow cleanup | Active unit variant, continuation notes, additional recipes, Chrome/Firefox/Safari confirmation, parser fixture |
| RecipeTin Eats | Rendered markup and Recipe JSON-LD directly inspected | Whole-page normal and print views captured in Codex in-app browser on Linux | Print workflow provisionally preferred; narrow cleanup/adapter | Additional recipes; Chrome, Firefox, Safari confirmation; parser fixture |
| King Arthur Baking | Rendered markup and Recipe JSON-LD directly inspected | Whole-page normal and print views captured in Codex in-app browser on Linux | Print workflow provisionally preferred; generic parser plus narrow cleanup | Additional recipes, dual-unit handling, Chrome/Firefox/Safari confirmation, parser fixture |
| Budget Bytes | Rendered markup and Recipe JSON-LD directly inspected | Whole-page normal and dedicated print-route views captured in Codex in-app browser on Linux | Print workflow provisionally preferred; generic parser plus narrow cleanup | Additional recipes, active unit/yield state, price annotations, Chrome/Firefox/Safari confirmation, parser fixture |
| Sally's Baking Addiction | Rendered markup and Recipe JSON-LD directly inspected | Whole-page normal and dedicated print-route views captured in Codex in-app browser on Linux | Print workflow provisionally preferred; generic parser plus boundary cleanup | Additional recipes, footnotes/notes, Chrome/Firefox/Safari confirmation, parser fixture |
| Bon Appétit | Rendered markup and Recipe JSON-LD directly inspected | Whole-page normal and card-focused selections captured in Codex in-app browser on Linux; no dedicated print action found | Whole-page workflow with proposed cleanup; adapter only if fixture-required; card-focused recovery | Access variants, Chrome, Firefox, Safari confirmation, parser fixture |
| Simply Recipes | Public rendered markup inspected; origin challenge prevented source confirmation | Whole-page normal and browser print-surface copies captured in Codex in-app browser on Linux | Normal-page workflow provisionally preferred; generic parser plus narrow cleanup | Additional recipes, card boundaries/image credits, Chrome/Firefox/Safari confirmation, parser fixture |

## Markup-research findings

The first research pass covered all twelve primary providers. It supports proceeding with clipboard import, but the parser must target the rendered recipe region; a page's Schema.org JSON-LD normally lives outside the selected rendered DOM and should not be expected in ordinary clipboard HTML.

Representative pages and the distinct fixture risks are:

- **Allrecipes:** [Banana Banana Bread](https://www.allrecipes.com/recipe/20144/banana-banana-bread/) exposes editorial ingredient lists before the actual card, scaling controls, and image captions between directions.
- **Food52:** [Busy Weeknight Bean Chili](https://food52.com/recipes/78043-busy-weeknight-bean-chili) uses useful ingredient groups but may place video or promotional content between the ingredient and direction regions.
- **Food Network:** [The Best Banana Bread](https://www.foodnetwork.com/recipes/food-network-kitchen/classic-banana-bread-3361704) repeats attribution and metadata, and one ordered direction may contain multiple paragraphs.
- **NYT Cooking:** [Chocolate Chip Cookies](https://cooking.nytimes.com/recipes/1015819-chocolate-chip-cookies) uses `Preparation`, separated step labels and numbers, repeated login prompts, and access-dependent behavior. Its inspected JSON-LD declares the example non-free; Noms processes only content the user can already select.
- **Serious Eats:** [Quick and Easy Chocolate Chip Cookies](https://www.seriouseats.com/bravetart-chocolate-chip-cookies-recipe) places a recipe card after substantial long-form editorial content and may interleave images or captions with directions. The previously recorded URL returned `Page Not Found` on 2026-08-20; the replacement was resolved through the site's visible search results.
- **BBC Good Food:** [Best chewy chocolate chip cookies](https://www.bbcgoodfood.com/recipes/chewy-chocolate-chip-cookies) has a strong card structure, but ingredient quantities, names and preparation notes may be split and unit tabs must not be combined.
- **RecipeTin Eats:** [The Chocolate Chip Cookies of My Dreams](https://www.recipetineats.com/the-chocolate-chip-cookies-of-my-dreams/) can contain both abbreviated and full instruction sections, creating a material duplicate-procedure risk.
- **King Arthur Baking:** [Classic Chocolate Chip Cookies](https://www.kingarthurbaking.com/recipes/classic-chocolate-chip-cookies-recipe) uses dual measurements and may insert technique or product promotions within the visible instruction flow.
- **Budget Bytes:** [Easy Homemade Meatballs](https://www.budgetbytes.com/homemade-meatballs/) includes per-ingredient prices, controls, grouped components, and a later illustrated duplicate of the card procedure.
- **Sally's Baking Addiction:** [Chewy Chocolate Chip Cookies](https://sallysbakingaddiction.com/chewy-chocolate-chip-cookies/) presents ingredient-like editorial bullets before a strongly structured card and uses footnote markers and long notes.
- **Bon Appétit:** [BA's Best Risotto](https://www.bonappetit.com/recipe/bas-best-risotto) splits amount, unit and ingredient text into adjacent nodes. Its inspected JSON-LD also merged substantial trailing editorial text into one instruction, proving that structured data cannot be trusted without validation.
- **Simply Recipes:** [Banana Bread](https://www.simplyrecipes.com/recipes/banana_bread/) has clean headings but interleaves step images and repeated photographer credits. Automated origin access was challenged even though the public rendered recipe was available.

The common generic rule should identify candidate regions by ordered heading pairs such as `Ingredients` followed by `Directions`, `Instructions`, `Method`, or `Preparation`, then score list density and boundary quality. It must not treat the entire page as one uniform recipe candidate.

The likely first adapter candidates are Allrecipes and RecipeTin Eats. Serious Eats, NYT Cooking, and Bon Appétit now have direct clipboard shapes that should first be exercised by the generic parser plus narrowly specified cleanup; they become adapters only if a synthetic fixture proves that path insufficient. The remaining providers should also start with the generic parser and add only fixture-proven cleanup rules. This classification remains provisional until real clipboard captures are run through a parser prototype.

Warnings must cover at least competing recipe regions, duplicate instruction variants, multiple unit or yield variants, fragmented ingredient lines, suspicious durations, missing source URL, discarded image or credit lines, and access shells with no selectable recipe.

## Product workflow decision

Use whole-page `Ctrl+A` then `Ctrl+C` (`Cmd+A` then `Cmd+C` on macOS) as the single provider-neutral first attempt. All twelve primary providers produced recognizable recipe signals through that workflow, so provider-specific feasibility does not block implementation. Import remains explicitly best effort: the editable preview may contain missing fields, preserved raw lines, competing candidates, or warnings.

Measured print views and focused selections remain optional recovery paths when a whole-page paste is oversized, ambiguous, or incomplete. The UI may suggest the appropriate recovery after parsing, but it should not require users to identify their provider or choose a special workflow before the first paste. Synthetic fixtures and the parser prototype will determine during implementation which providers are handled by generic rules, which need narrow adapters, and which shapes remain unsupported. These findings are compatibility targets rather than promises of perfect extraction.

## Direct clipboard observations

### Allrecipes whole-page copy

**Observed:** 2026-08-20 in a logged-out/public access state using the Codex in-app browser on Linux; its browser engine and exact version were not exposed to the capture page.

**Page:** [Banana Banana Bread](https://www.allrecipes.com/recipe/20144/banana-banana-bread/), normal view, full-page `Ctrl+A` then `Ctrl+C`.

The paste event exposed MIME types in this order:

1. `text/plain`
2. `text/html`

JavaScript string lengths were 17,159 code units for plain text and 680,044 code units for HTML. These are character-code-unit measurements, not encoded byte counts.

The HTML representation began with a rendered `<body>` subtree and retained extensive inline `style` attributes, provider classes and IDs, images, and links. It contained no `<script>` element and no detectable Schema.org `Recipe` or `application/ld+json` payload. The plain-text representation began with site navigation, breadcrumbs, ratings, description, attribution, controls, metadata, and editorial content before reaching the actual recipe card.

This confirms several implementation requirements:

- Normal clipboard HTML is useful because it retains structural and provider markers, but it must be aggressively sanitized and stripped of inline presentation data before parsing.
- Schema.org JSON-LD cannot be assumed to survive a normal page copy even when the source page publishes it.
- A generic parser must choose the later `Ingredients` → `Directions` card boundary instead of the earlier editorial `Banana Bread Ingredients` region.
- Payload limits must accommodate at least this approximately 680,000-code-unit normal-page HTML case or the UI must explicitly direct users to copy only the recipe card or print view. Final limits remain open until more providers are captured.
- Source URL remains a separate input; neither MIME representation should be assumed to carry a reliable canonical page URL.

The full third-party payload was kept only in the browser paste event. It was not written to disk or added to the repository.

### Allrecipes ingredients-through-directions copy

**Observed:** 2026-08-20 in the same browser and page context. The user selected from the `Ingredients` heading through the final `Enjoy!` direction and copied the selection.

The paste event again exposed `text/plain` followed by `text/html`. JavaScript string lengths fell to 1,487 code units for plain text and 34,782 code units for HTML—approximately 91% and 95% smaller than the whole-page representations respectively.

The HTML began at the provider's structured-ingredients container. It retained provider classes and IDs, extensive inline styles, images, and structural headings, but contained no links, scripts, or Schema.org JSON-LD. The plain-text representation cleanly retained the ingredient lines and ordered procedure, but also included scaling controls plus image alt text and photographer credits interleaved between directions.

This narrower selection is substantially easier to parse, but it omits the recipe title, description, source author, prep/cook/total times, and the richer yield metadata above the card. It preserves the in-card serving statement. Therefore:

- The UI should recommend copying from the title/metadata block through the final recipe direction when practical, not merely `Ingredients` onward.
- The parser should still accept an ingredients-through-directions selection and surface missing title, attribution, and time fields for manual completion.
- Allrecipes cleanup must discard scaling-control labels and distinguish image alt/credit lines from direction text without discarding image attribution when the user chooses to retain image references.
- Card-focused copying dramatically reduces the payload-size and boundary-detection burden, so it is worth presenting as recovery guidance after a noisy or oversized whole-page paste.

As with the whole-page observation, the full copied payload was not persisted.

### Allrecipes title-through-directions copy

**Observed:** 2026-08-20 in the same browser and page context. The user selected from the `Banana Banana Bread` page title through the final `Enjoy!` direction.

The paste event again exposed `text/plain` followed by `text/html`. JavaScript string lengths were 6,206 code units for plain text and 125,801 code units for HTML—approximately 64% and 82% smaller than the whole-page representations, but roughly four times larger than the ingredients-through-directions selection.

This selection retained the title, description, source author, test-kitchen attribution, prep/cook/total times, servings, yield, ingredients, and directions. It also retained ratings, controls, images, links, and the entire long-form editorial region placed between the recipe header and card, including the competing `Banana Bread Ingredients` heading and its explanatory ingredient list. It again contained inline styles but no scripts or Schema.org JSON-LD.

The workflow is complete but not clean. A contiguous browser selection cannot include the separated header and recipe card without also including the intervening article. Therefore:

- The generic region scorer and Allrecipes adapter remain necessary even when the user carefully follows title-through-directions guidance.
- The preview must identify competing ingredient regions and prefer the later `Ingredients` → `Directions` card while taking metadata from the earlier recipe header.
- A clean ingredients-through-directions paste plus manual title/source metadata may be easier for users than a large contiguous selection when no useful print view exists.
- Provider-specific copy guidance should be driven by measured results rather than one universal selection instruction.
- The Allrecipes print capture is now required to decide the recommended provider workflow.

The full copied payload was not persisted.

### Allrecipes print-view copy

**Observed:** 2026-08-20 in the same browser context. The Allrecipes `Print` action opened a selectable print-oriented view; the user selected all and copied it.

The paste event exposed `text/plain` followed by `text/html`. JavaScript string lengths were 1,486 code units for plain text and 37,640 code units for HTML—approximately 91% and 94% smaller than the whole-page representations. The HTML was only about 8% larger than the ingredients-through-directions selection.

The print copy retained the provider name, recipe title, description, submitted-by and test-kitchen attribution, prep/cook/total times, servings, yield, ingredients, and directions. It removed the long-form editorial article, the competing editorial ingredient list, scaling controls, and the image alt/credit lines interleaved between normal-page directions. The HTML still contained provider structure, inline styles, images, and links, but no scripts or Schema.org JSON-LD.

**Allrecipes decision:** recommend `Print` → select all → copy as the primary provider-specific workflow. The generic headings-and-lists parser should handle this shape after sanitization. Retain an Allrecipes adapter for normal-page and whole-page fallback, where it must join metadata from the recipe header to the later `Ingredients` → `Directions` card and ignore editorial competitors, scaling controls, and image-credit noise.

The UI should not imply that the print view supplies a canonical source URL. It must continue to offer the optional source-URL field and preserve Allrecipes attribution recovered from the copied content.

The full copied payload was not persisted.

### RecipeTin Eats print-view copy

**Observed:** 2026-08-20 using the Codex in-app browser on Linux; its browser engine and exact version were not exposed to the capture page.

**Page:** [The Chocolate Chip Cookies of My Dreams — print view](https://www.recipetineats.com/wprm_print/the-chocolate-chip-cookies-of-my-dreams), full-page `Ctrl+A` then `Ctrl+C`.

The paste event exposed `text/plain` followed by `text/html`. JavaScript string lengths were 8,159 UTF-16 code units for plain text and 75,855 UTF-16 code units for HTML; these are not encoded byte sizes. The HTML began with the rendered print-view `<body>` and retained inline styles, provider/plugin classes, images, and links. It contained no scripts and no detectable Schema.org Recipe or JSON-LD payload.

Combining the direct rendered-print-view inspection with the clipboard preview confirmed retention of the title, description, rating, prep/cook/total times, course, cuisine, keywords, servings, author, grouped ingredients, instructions, notes, and recipe image. The copy also retained print-page controls, the cups/metric selector, smaller/normal/larger yield controls, and both `ABBREVIATED RECIPE` and `FULL RECIPE` instruction variants exposed by the rendered print view.

**Provisional RecipeTin Eats decision for this captured shape:** the print view is a viable copy workflow, but it needs a narrow RecipeTin cleanup rule before the generic parser. Before removing styles, state classes, data attributes, and controls, the rule must resolve any reliably indicated active unit/yield variant into neutral semantic markers. It must preserve ingredient groups and notes and prefer a non-empty, structurally valid `FULL RECIPE` procedure when both variants are present. If the full procedure is absent or invalid, it may use a valid `ABBREVIATED RECIPE` instead. The preview must warn whenever it discards a competing instruction variant rather than silently merging both procedures.

This remains provisional until additional RecipeTin recipes, named-browser captures, and a synthetic parser fixture confirm the rule; the current unknown browser engine/version cannot close a Chrome, Firefox, or Safari matrix cell.

This print payload is roughly twice the Allrecipes print HTML size and more than five times its plain-text size, so print views are cleaner rather than necessarily small. Payload limits and structural-complexity checks must be based on the broader provider sample instead of the Allrecipes result alone.

As with the other captures, the full copied payload was not persisted. The optional source-URL field remains necessary because the copied print content should not be assumed to carry the canonical normal-page URL.

### RecipeTin Eats normal-page whole-page copy

**Observed:** 2026-08-20 in the same in-app browser and recipe context, normal view, full-page `Ctrl+A` then `Ctrl+C`.

The paste event exposed `text/plain` followed by `text/html`. JavaScript string lengths were 34,423 UTF-16 code units for plain text and 380,457 UTF-16 code units for HTML; these are not encoded byte sizes. The HTML retained inline styles, provider/plugin classes, images, and links, with no scripts or detectable Schema.org Recipe or JSON-LD payload.

The plain-text preview began with skip links, site navigation, account/search controls, breadcrumbs, title, author, comment count, share controls, publication dates, jump links, and long-form editorial content. The bounded preview repeated the recipe title three times before reaching the card content. The normal-page HTML was approximately five times the print HTML size, while normal-page text was approximately 4.2 times the print text size.

**Comparison decision for the captured page:** prefer `Print` → select all → copy over a normal-page whole-page copy. The print view substantially reduces navigation, editorial, comment, and boundary noise. Normal-page copying remains an accepted fallback that must locate the strongly structured WPRM recipe card instead of treating the first title or recipe-like editorial region as the extraction boundary. When a normal-page candidate contains the same unit/yield controls or competing instruction variants observed during direct rendered-page inspection, it must conditionally apply the corresponding cleanup and warnings rather than assume those structures are always present.

This normal-versus-print comparison strengthens the provisional recommendation for this page but does not yet establish provider-wide behavior. Additional RecipeTin recipes, named browsers, and a synthetic parser fixture remain required. The full copied payload was not persisted.

### Bon Appétit normal-page whole-page copy

**Observed:** 2026-08-20 using the Codex in-app browser on Linux; its browser engine and exact version were not exposed to the capture page.

**Page:** [BA's Best Risotto](https://www.bonappetit.com/recipe/bas-best-risotto), public normal view, full-page `Ctrl+A` then `Ctrl+C`.

The paste event exposed `text/plain` followed by `text/html`. JavaScript string lengths were 11,993 UTF-16 code units for plain text and 422,895 UTF-16 code units for HTML; these are not encoded byte sizes. The HTML retained inline styles, provider classes, images, and links, with no scripts or detectable Schema.org Recipe or JSON-LD payload.

The plain-text preview retained the recipe title, author, publication date, rating, image description and credit, description, and editorial technique content. Direct rendered-card inspection found a strong `Recipe information` → `Ingredients` → `Preparation` structure, but each ingredient quantity is a separate node from its combined unit/name text; unitless ingredients use an empty quantity node. Preparation uses `Step N` headings and coherent paragraphs. After Step 6, a `VARIATIONS` block, additional images, and subordinate topping recipes continue inside the same preparation list.

No link or button on the rendered page exposed a print action. Therefore the next comparison is a card-focused selection from `Recipe information` through Step 6. The proposed cleanup contract reconstructs adjacent quantity and unit/name nodes, preserves unitless ingredients when the amount node is empty or absent, stops the primary procedure before `VARIATIONS`, and avoids treating the merchandising block immediately before `Recipe information` as recipe content. A synthetic fixture must determine whether this requires a provider adapter or can be satisfied by the generic parser.

This page was publicly readable in the observed session, but that does not establish behavior for other access states or recipes. The full copied payload was not persisted.

### Bon Appétit card-focused copy

**Observed:** 2026-08-20 in the same browser and page context. The user selected from `Recipe information` through the final primary Step 6, stopping before `VARIATIONS`.

The paste event exposed `text/plain` followed by `text/html`. JavaScript string lengths were 6,798 UTF-16 code units for plain text and 54,638 UTF-16 code units for HTML; these are not encoded byte sizes. Compared with the whole-page capture, this reduced HTML by approximately 87% and plain text by approximately 43%.

The copy retained total time, yield, ingredients, and all six primary preparation steps while excluding the earlier merchandising and editorial regions and the later variations. It omitted the page title, author, publication date, rating, description, hero image, and image credit because those fields occur above `Recipe information`.

The plain text made the ingredient-fragmentation requirement concrete: numeric quantities were separated from combined unit/name text by blank lines, while the unitless `Freshly ground black pepper` line had no preceding quantity. It also inserted `Want to turn this into a meal plan?` and `Ask AI` between ingredients and `Preparation`. HTML appears to retain sufficient adjacent structure and provider markers for reconstruction after removing the control block, but a synthetic fixture must confirm deterministic behavior. A plain-text fallback needs a Bon Appétit-specific line-joining rule only if the generic text parser fails that fixture, and it must preserve unitless lines.

**Bon Appétit decision for the captured page:** provisionally use whole-page `Ctrl+A` → `Ctrl+C` as the primary workflow because it is simpler and retains attribution and descriptive metadata, subject to the payload-limit and parser-fixture checks. The proposed cleanup contract locates the later `Recipe information` → `Ingredients` → `Preparation` card, reconstructs split ingredients, removes `Ask AI`/meal-plan and merchandising controls, and stops the primary procedure before `VARIATIONS`; it becomes a Bon Appétit adapter only if the fixture proves the generic parser insufficient. Offer card-focused copying from `Recipe information` through the final primary step as recovery guidance for oversized or ambiguous imports, while warning that title, attribution, description, and image metadata require manual completion.

This remains provisional provider guidance until additional recipes, access states, named browsers, and a synthetic parser fixture confirm the boundaries. The full copied payload was not persisted.

### NYT Cooking normal-page copy and logged-out print behavior

**Observed:** 2026-08-20 using the Codex in-app browser on Linux; its browser engine and exact version were not exposed to the capture page.

**Page:** [Chocolate Chip Cookies](https://cooking.nytimes.com/recipes/1015819-chocolate-chip-cookies), normal view, full-page `Ctrl+A` then `Ctrl+C` in the observed access state.

The paste event exposed `text/plain` and `text/html`. JavaScript string lengths were 1,335 UTF-16 code units for plain text and 34,947 UTF-16 code units for HTML; these are not encoded byte sizes. The HTML began at the rendered recipe page body, retained inline styles, images, and links, and contained ingredient and `Preparation` signals. It contained no scripts and no detectable Schema.org Recipe or JSON-LD payload. The bounded plain-text preview retained the title, recipe/adaptation attribution, update date, media credit, total time, rating, comment count, and the beginning of the description. The capture inspector did not persist the complete payload, so a synthetic fixture is still required to verify ingredient and preparation recovery rather than inferring it from page markup alone.

The visible `Print Recipe` action was also tested in the same logged-out state. It redirected to the New York Times sign-in flow instead of exposing a selectable print view. Noms must not bypass that access boundary or make print-view copying a prerequisite for NYT imports. A signed-in/subscriber capture remains an optional compatibility check, not a release dependency for evaluating the normal-page workflow.

**Provisional NYT Cooking decision:** treat normal-page `Ctrl+A` → `Ctrl+C` as the anonymous workflow candidate and feed the sanitized HTML through the generic `Ingredients` → `Preparation` region scorer. Confirm that recommendation only after a synthetic fixture recovers the ingredients and preparation steps from the captured shape. Preserve recipe/adaptation attribution and warn when only introductory metadata is recoverable from plain text. Approve a narrow NYT adapter only if the synthetic HTML fixture demonstrates a deterministic generic-parser failure. The preview should continue to request an optional source URL because clipboard content does not establish the canonical page URL.

This remains provisional provider guidance until the fixture, additional recipes and access states, and named-browser captures are complete. The full copied payload was not persisted.

### Serious Eats normal-page and print-route copies

**Observed:** 2026-08-20 using the Codex in-app browser on Linux; its browser engine and exact version were not exposed to the capture page.

**Page:** [Quick and Easy Chocolate Chip Cookies](https://www.seriouseats.com/bravetart-chocolate-chip-cookies-recipe), full-page `Ctrl+A` then `Ctrl+C` from the normal view and from the site's `?print` route after dismissing the browser print surface. The previously recorded sample URL returned `Page Not Found`; the current URL was resolved through the site's visible search results.

The normal-page paste exposed `text/plain` and `text/html`: 19,506 UTF-16 code units of plain text and 436,583 UTF-16 code units of HTML. The HTML began at the full rendered page body, retained inline styles, images, and links, and contained ingredient and instruction signals, but no scripts or detectable Schema.org Recipe/JSON-LD payload. Its plain-text preview began with navigation and promotional text before the title, rating, reviews, description, author/date, print/share controls, article table of contents, and long-form editorial content. This confirms that the generic parser must locate the later recipe card instead of treating the first recipe-related article region as the candidate.

The print-route paste also exposed `text/plain` and `text/html`: 3,768 UTF-16 code units of plain text and 52,102 UTF-16 code units of HTML. Relative to the normal capture, this reduced HTML by about 88% and plain text by about 81%. Its plain text began directly with provider, title, description, author/date, prep/cook/total times, servings, yield, and `Ingredients`; the payload also contained direction signals. The HTML retained inline styles and links but no images, scripts, or detectable Schema.org Recipe/JSON-LD payload.

The inspector retained only bounded previews, so a synthetic fixture must still prove complete ingredient and instruction recovery. It must also confirm that the print route's lack of images is represented as a missing optional image rather than an import failure.

**Provisional Serious Eats decision:** recommend `Print` → dismiss the browser print surface if necessary → select all → copy. Feed the sanitized print shape through the generic headings-and-lists parser, retaining attribution and times and warning that the image is unavailable from this workflow. Keep normal-page copying as a fallback that scores the later recipe card after long-form editorial content and discards navigation, article prose, figures, and captions. Approve a provider adapter only if a synthetic fixture demonstrates a deterministic failure that narrow boundary cleanup cannot resolve.

This remains provisional provider guidance until the fixture, additional recipes, named-browser captures, and the native print-surface interaction are confirmed. The full copied payload was not persisted.

### BBC Good Food normal-page copy

**Observed:** 2026-08-20 using the Codex in-app browser on Linux; its browser engine and exact version were not exposed to the capture page.

**Page:** [Best chewy chocolate chip cookies](https://www.bbcgoodfood.com/recipes/chewy-chocolate-chip-cookies), normal view, full-page `Ctrl+A` then `Ctrl+C`.

The paste event exposed `text/plain` and `text/html`: 5,275 UTF-16 code units of plain text and 357,703 UTF-16 code units of HTML. The HTML began at the full rendered page body, retained inline styles, images, and links, and contained ingredient and method signals, but no scripts or detectable Schema.org Recipe/JSON-LD payload.

The bounded plain-text preview retained the title, author, yield, difficulty, prep and cook times, optional chilling note, rating/count, description, dietary/storage labels, ingredients, and the beginning of `Method`. It also contained site navigation, app promotion, advertisements, a media-carousel status line, nutrition/unit controls, and a cook-mode prompt. Ingredient lines can continue onto following lines, such as `150g butter` followed by `softened`, and alternatives can appear on a continuation line, so the text parser must preserve those lines for preview rather than treating each as a separate ingredient without evidence.

No print-labelled link, button, or form appeared in the rendered DOM snapshot. Browser-native printing may change presentation but does not provide a stable, provider-owned selectable route, so it is not proposed as the import workflow.

**Provisional BBC Good Food decision:** subject to payload-limit and parser-performance checks, use normal-page `Ctrl+A` → `Ctrl+C`, locate the `Ingredients` → `Method` region with the generic parser, and apply narrow cleanup for navigation, ads, app/cook-mode prompts, media carousels, and nutrition controls. Before sanitization removes state markers, retain only a reliably active unit variant; never merge metric and alternate units. If active state cannot be established, preserve the variants as competing candidates and warn instead of choosing or discarding one. Preserve ingredient continuation lines and flag uncertain joins in the preview. Add a provider adapter only if the synthetic fixture proves these generic rules insufficient.

This remains provisional provider guidance until the correctness fixture, payload-limit and parser-performance checks, additional recipes, active-unit variants, continuation notes, and named-browser captures are complete. The full copied payload was not persisted.

### King Arthur Baking normal-page and print-view copies

**Observed:** 2026-08-20 using the Codex in-app browser on Linux; its browser engine and exact version were not exposed to the capture page.

**Page:** [Classic Chocolate Chip Cookies](https://www.kingarthurbaking.com/recipes/classic-chocolate-chip-cookies-recipe), full-page `Ctrl+A` then `Ctrl+C` from the normal page and from the selectable print document exposed after the site's `Print` action opened the browser print surface.

The normal-page paste exposed `text/plain` and `text/html`: 8,378 UTF-16 code units of plain text and 306,605 UTF-16 code units of HTML. The HTML began at the full rendered page body, retained inline styles, images, and links, and contained ingredient and direction signals, but no scripts or detectable Schema.org Recipe/JSON-LD payload. Its bounded plain-text preview began with site navigation and promotional content before the title, author, rating, description, prep/bake/total times, yield, save control, and repeated image-alt text.

The print-document paste also exposed `text/plain` and `text/html`: 3,047 UTF-16 code units of plain text and 28,966 UTF-16 code units of HTML. Relative to the normal capture, this reduced HTML by about 91% and plain text by about 64%. Its text began directly with the provider, title, description, times, yield, image description, and `Ingredients`; the payload also retained direction signals, image elements or references, and links while excluding scripts and detectable Schema.org Recipe/JSON-LD. The copied HTML exposed a provider print path, `/recipes/print/16646`, even though the user reached it through the browser print surface rather than a separately visible page navigation. That path is a provider-detection hint, not the canonical source URL; the import preview must still accept an optional source URL.

The ingredient preview preserved paired volume and weight measurements such as `2/3 cup (142g)` and `8 tablespoons (113g)`. These are one ingredient representation, not competing unit variants, and must remain together. The inspector retained only bounded previews, so a synthetic fixture must still prove complete ingredient and instruction recovery and determine whether any promotions or baker's notes remain inside the print procedure.

**Provisional King Arthur Baking decision:** recommend activating `Print`, then selecting all and copying from the selectable provider print document exposed by the resulting print surface. If a browser exposes only non-selectable native print UI, use normal-page copying rather than requiring print copying. Feed the sanitized print shape through the generic headings-and-lists parser, preserve parenthesized metric equivalents on the same ingredient line, and retain only safe image references, alternative text, and credits when policy and payload limits permit; parsing must not fetch remote media. The normal-page fallback locates the later recipe region and removes navigation, promotions, controls, and repeated image descriptions. Add provider-specific cleanup only if a synthetic fixture demonstrates a stable boundary or duplication failure; do not create an independent King Arthur parser.

This remains provisional provider guidance until the fixture, additional recipes, dual-unit cases, named-browser captures, and print-surface interaction are confirmed. The full copied payload was not persisted.

### Budget Bytes normal-page and print-route copies

**Observed:** 2026-08-20 in a logged-out/public access state using the Codex in-app browser on Linux; its browser engine and exact version were not exposed to the capture page.

**Page:** [Easy Homemade Meatballs](https://www.budgetbytes.com/homemade-meatballs/), full-page `Ctrl+A` then `Ctrl+C` from the normal page and from the dedicated print route linked by the visible `Print Recipe` action: `/wprm_print/homemade-meatballs-recipe-2`.

The normal-page paste exposed `text/plain` and `text/html`: 24,179 UTF-16 code units of plain text and 488,922 UTF-16 code units of HTML. The HTML began at the full rendered page body, retained inline styles, images, and links, and contained ingredient and direction signals, but no scripts or detectable Schema.org Recipe/JSON-LD payload. Its bounded plain-text preview retained the title, author, publication date, rating, prep/cook times, servings, recipe and per-serving prices, description, and image description, but began with navigation and continued through extensive editorial material before the recipe card. Direct rendered inspection confirmed that the card contains grouped ingredients, equipment, multiple instruction sections, notes, nutrition, unit and serving controls, and per-ingredient price annotations; a later step-by-step section risks duplicating the card procedure in whole-page imports.

The dedicated print-route paste also exposed `text/plain` and `text/html`: 5,279 UTF-16 code units of plain text and 80,243 UTF-16 code units of HTML. Relative to the normal capture, this reduced HTML by about 84% and plain text by about 78%. Its bounded text began with print options, serving and unit controls, rating, title, description, prep/cook/total times, course, cuisine, total cost, servings, calories, author, equipment, and `Ingredients`; the payload also contained direction signals. It retained inline styles and links but no image elements, scripts, or detectable Schema.org Recipe/JSON-LD payload. The `/wprm_print/...` route is at most a provider-detection hint, not the canonical recipe source; the import preview still needs the optional source-URL field.

The copied print text exposed `US Customary` and `Metric` controls, smaller/normal/larger yield controls, and prices embedded in ingredient lines. Before sanitization removes state classes, attributes, or controls, the importer must retain only a reliably active unit and yield representation. It must never combine measurement variants. Prices are optional provider annotations rather than part of the ingredient name; a narrow cleanup may separate them into import-only metadata when deterministic, otherwise preserve the original line for user review. The inspector retained only bounded previews, so a synthetic fixture must still prove complete grouped-ingredient and multi-section instruction recovery, notes handling, and absence of duplicated procedures.

**Provisional Budget Bytes decision:** recommend the visible `Print Recipe` action followed by full-page select-all and copy from the dedicated print route. Feed the sanitized shape through the generic headings-and-lists parser, preserving equipment, ingredient groups, instruction section headings, and notes for preview. Resolve active unit and yield state before sanitization when reliable; otherwise preserve competing candidates and warn. Warn that no image is available from the print workflow. Keep normal-page copying as a fallback that scores the recipe card and excludes navigation, editorial sections, social/email controls, reviews, nutrition controls, and the later illustrated step-by-step duplicate. Add provider-specific cleanup only if a synthetic fixture proves stable price, state, or boundary rules are necessary; do not create an independent Budget Bytes parser.

This remains provisional provider guidance until the fixture, additional recipes, active unit and yield cases, price annotations, named-browser captures, and print-link behavior are confirmed. The full copied payload was not persisted.

### Sally's Baking Addiction normal-page and print-route copies

**Observed:** 2026-08-20 in a logged-out/public access state using the Codex in-app browser on Linux; its browser engine and exact version were not exposed to the capture page.

**Page:** [Chewy Chocolate Chip Cookies](https://sallysbakingaddiction.com/chewy-chocolate-chip-cookies/), full-page `Ctrl+A` then `Ctrl+C` from the normal page and from the dedicated `/chewy-chocolate-chip-cookies/print/70437/` route linked by the visible `Print Recipe` action.

The normal-page paste exposed `text/plain` and `text/html`: 21,169 UTF-16 code units of plain text and 315,389 UTF-16 code units of HTML. The HTML began at the full rendered page body, retained inline styles, images, and links, and contained ingredient and direction signals, but no scripts or detectable Schema.org Recipe/JSON-LD payload. Its bounded text began with cookbook promotion and site navigation before the title, comment count, author, update date, description, image description, and affiliate disclosure. Direct rendered inspection confirmed extensive editorial ingredient discussion, success tips, FAQs, recommended-tool links, freezing guidance, and summary bullets before the later recipe card. These earlier ingredient-like lists and numbered prose are competing regions that whole-page parsing must not mistake for the recipe.

The dedicated print-route paste also exposed `text/plain` and `text/html`: 4,977 UTF-16 code units of plain text and 50,790 UTF-16 code units of HTML. Relative to the normal capture, this reduced HTML by about 84% and plain text by about 76%. Its bounded text began with an image description followed directly by title, rating, review count, author, prep/cook/total times, an alternative yield (`16 XL cookies or 20 medium/large cookies`), description, and `Ingredients`; the payload also contained direction signals. It retained inline styles, image elements or references, and links while excluding scripts and detectable Schema.org Recipe/JSON-LD. The print payload's body classes indicated that images, description, and notes were enabled for this print representation, but complete ordered ingredient, instruction, and notes recovery remains fixture work because the inspector retained only bounded previews.

Ingredient lines preserve paired volume and weight measurements, internal alternatives, and footnote markers such as `cornstarch*`. These markers must remain attached to the ingredient until the corresponding note can be recovered; the importer must not discard them as decoration. When no matching note is recovered, retain the raw marker and warn in the preview. The alternative yield must remain raw text unless the candidate model gains an explicit multi-option representation; it must not be normalized as a numeric range. The print path is at most a provider-detection hint, not the canonical recipe source, so the import preview still needs the optional source-URL field.

**Provisional Sally's Baking Addiction decision:** recommend the visible `Print Recipe` action followed by full-page select-all and copy from the dedicated print route. Feed the sanitized print shape through the generic headings-and-lists parser, preserving paired measurements, alternatives, raw multi-option yield text, image references/alternative text/credits, footnote markers, and notes for preview. Warn when a marker has no recovered note. Parsing must not fetch remote images. Keep normal-page copying as a fallback that scores the later recipe card and excludes navigation, promotions, affiliate/tool links, editorial ingredient lists, tips, FAQs, testimonials, and other numbered prose. Add provider-specific cleanup only if a synthetic fixture proves stable boundary or footnote rules are necessary; do not create an independent Sally's parser.

This remains provisional provider guidance until the fixture, additional recipes, footnote and notes cases, named-browser captures, and print-link behavior are confirmed. The full copied payload was not persisted.

### Simply Recipes normal-page and browser print-surface copies

**Observed:** 2026-08-20 in a logged-out/public access state using the Codex in-app browser on Linux; its browser engine and exact version were not exposed to the capture page.

**Page:** [Banana Bread](https://www.simplyrecipes.com/recipes/banana_bread/), full-page `Ctrl+A` then `Ctrl+C` from the normal page and after activating the visible `PRINT` action and performing the same keys while browser print UI was shown.

The normal-page paste exposed `text/plain` and `text/html`: 14,623 UTF-16 code units of plain text and 465,911 UTF-16 code units of HTML. The HTML began at the full rendered page body, retained inline styles, images, and links, and contained ingredient and direction signals, but no scripts or detectable Schema.org Recipe/JSON-LD payload. Its bounded text retained the title, rating and review counts, description, author, update date, controls, table-of-contents headings, editorial introduction, image description, and image credit before the recipe card.

The visible `PRINT` action opened browser print UI without navigating to a provider-owned print URL. The user then performed `Ctrl+A` and `Ctrl+C` while that UI was shown. The subsequent paste also exposed `text/plain` and `text/html`: the same 14,623 UTF-16 code units of plain text and 465,862 UTF-16 code units of HTML. The bounded previews and all recorded signals matched the normal-page capture; the 49-code-unit HTML difference is immaterial at this payload size. The retained evidence cannot establish whether native print UI supplied selectable content or focus remained on the underlying page, but it establishes that no distinct selectable provider print document or materially cleaner payload was observed. This is a workflow observation, not a claim that every browser's print clipboard behavior is identical.

Direct rendered inspection had already identified clean `Ingredients`, `Method`, and `Recipe Notes` headings inside the article, but step images and repeated photographer credits can appear between method steps. The parser must attach image alternative text and credits to optional media metadata or discard them with a warning; it must not turn either into instruction text. The inspector retained only bounded previews, so a synthetic fixture must prove complete ordered ingredient, method, and notes recovery and correct media-boundary behavior.

**Provisional Simply Recipes decision:** use normal-page full-page copy as the current workflow candidate and feed the sanitized HTML through the generic recipe-region scorer, subject to the payload-size and parser-performance gates for approximately 466,000 UTF-16 code units of HTML. Locate the later `Ingredients` → `Method` → `Recipe Notes` region, preserve author/update metadata and safe image references/alternative text/credits for preview, and never fetch remote media during parsing. Remove navigation, app promotion, controls, table-of-contents/editorial content, nutrition UI, reviews, and unrelated images. Do not recommend browser print copying unless a named-browser capture demonstrates a distinct and materially cleaner representation. Before full-page copying becomes provider-wide guidance, compare it with a focused selection from the recipe-card title through `Recipe Notes`. Add provider-specific cleanup only if a synthetic fixture proves stable media-credit or boundary rules are necessary; do not create an independent Simply Recipes parser.

This remains provisional provider guidance until the focused-selection comparison, fixture, payload/performance checks, additional recipes, image-credit cases, named-browser captures, and print behavior are confirmed. The full copied payload was not persisted.

### Food52 normal-page and browser print-interaction copies

**Observed:** 2026-08-20 in a logged-out/public access state using the Codex in-app browser on Linux; its browser engine and exact version were not exposed to the capture page.

**Page:** [Busy Weeknight Bean Chili](https://food52.com/recipes/78043-busy-weeknight-bean-chili), full-page `Ctrl+A` then `Ctrl+C` from the normal page and after activating the visible `Print` action and performing the same keys while browser print UI was shown.

The normal-page paste exposed `text/plain` and `text/html`: 2,638 UTF-16 code units of plain text and 917,095 UTF-16 code units of HTML. The HTML began at the full rendered page body with an unusually large inline Tailwind-style property set, retained images and links, and contained ingredient and direction signals, but no scripts or detectable Schema.org Recipe/JSON-LD payload. Its bounded text began with privacy, search, navigation, and cart content before retaining title, rating, author, image credit, times, yield, description, ingredients, and directions. Ingredient quantity, unit, and name fragments collapsed together in lines such as `6 tablespoonunsalted butter, divided` and `2(15.5-ounce) cans kidney beans, divided`.

The visible `Print` action opened browser print UI without exposing a provider-owned print URL to the captured tab. The user then performed `Ctrl+A` and `Ctrl+C` while that UI was shown. The subsequent paste again exposed `text/plain` and `text/html`: 2,639 UTF-16 code units of plain text and 947,177 UTF-16 code units of HTML. The bounded previews and all recorded structure signals matched the normal-page capture. The print-surface capture therefore added about 30,000 HTML code units rather than reducing the payload, while changing plain text by only one code unit. The retained evidence cannot establish whether native print UI supplied selectable content or focus remained on the underlying page, but it establishes that no distinct or materially cleaner print representation was observed.

The parser must locate the title/metadata and `Ingredients` → `Directions` recipe region after navigation, preserve author, displayed rating/count, and image credit for preview, remove interactive rating controls, and reconstruct ingredient spacing from DOM boundaries before flattening only when a fixture proves the rule deterministic. When HTML boundaries are absent or insufficient, the plain-text fallback must preserve the collapsed raw ingredient line and warn rather than guessing inserted spaces. Promotional video content or explanatory links between recipe regions must not become ingredient or instruction text. Because the inspector retained only bounded previews, a synthetic fixture must prove complete ordered ingredient and direction recovery, grouped-section handling, and safe recipe boundaries.

**Provisional Food52 decision:** use normal-page copying as the current workflow candidate and feed sanitized HTML through the generic recipe-region scorer, subject to payload-size and parser-performance gates for approximately 917,000 UTF-16 code units of HTML. Remove navigation, cart UI, interactive controls, video/promotional blocks, and unrelated links or images while preserving displayed rating metadata, recipe attribution, and safe image references/alternative text/credits without fetching remote media. Preserve collapsed raw ingredient lines with warnings unless fixture-proven DOM boundaries support deterministic reconstruction. Do not recommend browser print copying unless a named-browser capture demonstrates a distinct and materially cleaner representation. Compare full-page copying with a focused selection from the recipe title through the final direction before provider-wide guidance. Add provider-specific cleanup only if a synthetic fixture proves stable fragment-joining or boundary rules are necessary; do not create an independent Food52 parser.

This remains provisional provider guidance until the focused-selection comparison, fixture, payload/performance checks, additional recipes, grouped-section and video-boundary cases, named-browser captures, and print behavior are confirmed. The full copied payload was not persisted.

### Food Network normal-page and browser print-interaction copies

**Observed:** 2026-08-20 in a logged-out/public access state using the Codex in-app browser on Linux; its browser engine and exact version were not exposed to the capture page.

**Page:** [The Best Banana Bread](https://www.foodnetwork.com/recipes/food-network-kitchen/classic-banana-bread-3361704), full-page `Ctrl+A` then `Ctrl+C` from the normal page and after activating the visible `Print Recipe` action and performing the same keys while browser print UI was shown.

The normal-page paste exposed `text/plain` and `text/html`: 5,077 UTF-16 code units of plain text and 227,778 UTF-16 code units of HTML. The HTML began at the full rendered recipe-page body, retained images and links, and contained ingredient and direction signals, but no scripts or detectable Schema.org Recipe/JSON-LD payload. Its bounded text began with navigation and breadcrumbs before retaining recipe attribution, title, displayed rating/review count, difficulty, total and active times, yield, nutrition control, description, ingredients, and directions. Direct rendered inspection confirmed a compact `Ingredients` → `Directions` region followed by `Cook’s Note`, with checked ingredient controls, shopping-list actions, and substantial unrelated content after the recipe.

The visible `Print Recipe` action opened browser print UI without changing the captured tab to a provider-owned print URL. The user then performed `Ctrl+A` and `Ctrl+C` while that UI was shown. The subsequent paste again exposed `text/plain` and `text/html`: the same 5,077 UTF-16 code units of plain text and 229,361 UTF-16 code units of HTML. The bounded previews and all recorded structure signals matched the normal-page capture; the print-interaction capture added 1,583 HTML code units rather than reducing the payload. The retained evidence cannot establish whether native print UI supplied selectable content or focus remained on the underlying page, but it establishes that no distinct or materially cleaner print representation was observed.

The captured page displayed `Active: 20 hr` alongside `Total: 1 hr 30 min (includes cooling time)`. This inconsistency must be preserved as raw source data and surfaced as a suspicious-duration warning; the importer must not silently reinterpret `hr` as `min`. Ingredient checkboxes and `Deselect All` are interactive state, not ingredient content. The parser should preserve attribution, displayed rating/count, difficulty, raw times, yield, description, ordered ingredients, ordered directions, and `Cook’s Note`, while removing navigation, previous/next links, print/save/share/meal-plan controls, shopping-list actions, private-notes UI, reviews, recommendations, and unrelated media. A direction list item may contain multiple sentences or paragraphs and must remain one ordered step unless explicit substructure is present.

**Provisional Food Network decision:** use normal-page copying as the current workflow candidate and feed sanitized HTML through the generic recipe-region scorer. Do not recommend browser print copying unless a named-browser capture demonstrates a distinct and materially cleaner representation. Score the attribution/title/metadata plus `Ingredients` → `Directions` → `Cook’s Note` region, preserve questionable values verbatim with warnings, and remove interactive controls without discarding the displayed metadata they label. Compare full-page copying with a focused selection from attribution/title through `Cook’s Note` before provider-wide guidance. Add provider-specific cleanup only if a synthetic fixture proves stable control-removal, note-boundary, or multi-paragraph-step rules are necessary; do not create an independent Food Network parser.

This remains provisional provider guidance until the focused-selection comparison, fixture, additional recipes, suspicious-duration and multi-paragraph-step cases, named-browser captures, and print behavior are confirmed. The full copied payload was not persisted.

With this capture, all twelve primary providers have at least one direct in-app-browser clipboard observation. This completes the first-pass provider capture matrix and is sufficient to begin the best-effort implementation. It does not close compatibility testing: synthetic fixtures and parser-prototype results will measure feasibility during implementation, while focused selections where called out, additional recipes and access states, and named Chrome/Firefox/Safari captures remain recovery and release-verification work.

## Initial implementation constraints discovered

- The Web Clipboard Event API exposes paste data through `ClipboardEvent.clipboardData`; browsers commonly provide `text/html` and `text/plain`, but the user agent or source page may sanitize or replace either representation.
- A detached document is not by itself a no-network guarantee. Resource-bearing attributes must be neutralized before insertion into any parser DOM, or parsing must use a proven non-fetching HTML parser.
- Noms already has WASM bindings for `DataTransfer`, but its `web-sys` feature list does not yet include the clipboard event types needed to capture both paste representations.
- The current Noms recipe model stores flat ingredients and recursive instruction steps. It has no portable representation for ingredient sections, textual or range-based yield, total time distinct from prep/cook time, source URL, source author, imported attribution, or raw uncertain lines.
- Import must therefore produce a separate candidate model before mapping into `RecipeEditorState`; mapping provider content directly into the persistence model would discard evidence before review.
- Clipboard payloads do not reliably identify the source page. The import screen needs an optional source-URL field; provider inference from retained links, image hosts, classes, IDs, or data attributes is advisory and must never manufacture attribution.

## Provisional candidate contract

The spike should refine an import-only candidate with:

- raw clipboard MIME representations retained only for the active preview session;
- normalized source URL and provider identity when supplied or recoverable;
- optional title, description, source author, attribution, times, and yield as original text plus parsed values where safe;
- ordered ingredient and instruction sections containing their original lines;
- optional image references that are never fetched during parsing;
- unclassified source lines;
- field-level provenance, parser identifier, and warnings;
- no visibility or ownership choice other than the eventual private-draft default.

The candidate is not the canonical export snapshot and must not be persisted as a recipe until the user confirms the editor preview.

## Remaining hands-on work

- Capture the provider/browser clipboard matrix using a small local paste inspector that reports MIME types and bounded previews without uploading or saving clipboard contents.
- Convert the smallest distinct HTML and text shapes into synthetic fixtures.
- Run the proposed generic parsers against those fixtures and classify failures.
- Approve source adapters only for failures with stable provider markers and narrow deterministic fixes.
- Turn the confirmed candidate contract, parser tiers, warnings, and limits into NOMS-016 implementation checkpoints.
