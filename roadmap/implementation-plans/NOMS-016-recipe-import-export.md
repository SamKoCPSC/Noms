# NOMS-016: Recipe Import & Export — Implementation Plan

**Status:** Discovery in progress — clipboard compatibility spike started
**Created:** 2026-08-15
**Depends on:** Current recipe, draft, versioning, collection, and image models
**Coordination:** NOMS-017 structured metadata must define how imported and exported taxonomy fields are represented.
**Approach:** Start with user-initiated clipboard import and an editable private-draft preview. Parse pasted HTML and plain text deterministically, refine compatibility for selected recipe-site layouts, and defer NLP and browser-assisted clipping until the simpler workflow is validated. Schema.org remains the preferred contract for later URL import and interoperable export; Noms JSON remains the lossless portability format.

---

## Intended outcome

Users can bring recipes into Noms without retyping them and can take their recipe data out in durable, documented formats.

## Proposed planning scope

- Import recipe content that a user explicitly copies and pastes from another website, accepting both clipboard HTML and plain text.
- Parse common recipe headings, metadata, ingredient lists, instruction lists, and sections without requiring NLP.
- Research representative clipboard output from selected recipe sites and add small source-specific refinements only where the generic parser is insufficient.
- Present parsed data for review and correction before saving it as a draft.
- Preserve source URL and attribution when supplied; imported recipes must default to private.
- Define clipboard sanitization, parsing, payload-size, remote-resource, and attribution rules.
- Define a later URL-import path that prioritizes Schema.org `Recipe` JSON-LD and applies safe fetch, timeout, redirect, SSRF, access-policy, and attribution rules.
- Export individual recipes and collections in a documented structured format.
- Export interoperable Schema.org `Recipe` JSON-LD and embed equivalent JSON-LD in public or print-friendly recipe HTML.
- Export a versioned Noms JSON format for lossless round trips of Noms-specific data.
- Provide print-friendly and PDF-oriented output without making PDF the canonical data format.
- Define selectable image-export modes that can retain references or package image bytes explicitly, without copying remote media implicitly.

## Initial import milestone: copy and paste

The first usable import flow should avoid server-side fetching and browser automation and use one provider-neutral instruction:

1. The user opens the recipe page and presses `Ctrl+A` then `Ctrl+C` (`Cmd+A` then `Cmd+C` on macOS) to copy the whole rendered page.
2. The user pastes into a dedicated Noms import screen.
3. The client captures both `text/html` and `text/plain` clipboard representations when available.
4. Noms sanitizes the input, extracts a recipe candidate, and reports ambiguous or missing fields.
5. The user reviews and corrects the candidate in the recipe editor.
6. Noms saves only after confirmation, as a private draft with source attribution.

Whole-page copying is intentionally best effort. The first-pass spike found recognizable recipe signals in whole-page copies from all twelve primary providers, which is sufficient to begin implementation, but it does not promise complete extraction for every page or provider. Print views and focused recipe-card selections remain optional recovery guidance when the initial paste is oversized, ambiguous, or incomplete; users should not need provider-specific instructions before their first attempt. Provider feasibility and the boundary between generic parsing and narrow adapters will be tested during implementation with synthetic fixtures and parser-prototype results. Unsupported or incomplete shapes must fail into an editable preview with preserved raw lines and warnings rather than block the import feature or silently invent data.

The clipboard importer should use a layered, deterministic pipeline:

1. Capture the available clipboard MIME representations and an optional user-supplied source URL.
2. Enforce raw payload and structural limits, then tokenize or parse HTML without network activity; neutralize resource-bearing attributes before any DOM insertion.
3. Detect advisory provider markers from the bounded raw representation or non-fetching parse tree.
4. When a recognized page shape encodes active state in classes, attributes, visibility, or controls, run a bounded provider pre-normalization that converts only the required state into neutral semantic markers.
5. Destructively sanitize into an allowlisted structural DOM; retain useful classes, IDs, links, and data attributes only when they cannot execute or load content.
6. Apply the recognized-source adapter to the sanitized, pre-normalized structure when required.
7. Otherwise, locate and score candidate recipe regions from ordered heading pairs, list density, and clean boundaries, then apply the generic HTML parser.
8. Fall back to a generic plain-text parser.
9. Preserve uncertain source lines and competing candidates rather than destructively guessing.

Initial extraction rules should recognize:

- Recipe title and description.
- Prep time, cook time, total time, servings, and yield.
- `Ingredients`, `Directions`, `Instructions`, and `Preparation` headings.
- Bulleted ingredient lists and numbered instruction lists.
- Ingredient and instruction section headings, including subordinate sauces or components.
- Source URL and author when present in the clipboard content or supplied separately.

Ingredient normalization must be conservative. A line may be split into `amount`, `unit`, and `name` only when the result is sufficiently unambiguous; otherwise the complete original line should be retained as the ingredient name for user correction. Every parser should return warnings or confidence information suitable for the preview UI.

Parsing must produce an import-only candidate before mapping fields into `RecipeEditorState`. The candidate retains ordered ingredient and instruction sections, original lines, textual and parsed time/yield values, unclassified lines, source and field provenance, image references, parser identity, competing candidates, and warnings for the active preview session. This intermediate model is required because the current recipe persistence model cannot represent every observed provider shape without loss. Raw clipboard payloads are discarded when the preview ends and are never saved as recipe content.

The paste screen must expose an optional source URL field. A normal clipboard selection does not reliably contain the page's canonical URL; links, image hosts, or retained provider markers may suggest a provider but must not silently manufacture source attribution.

### Clipboard compatibility research

The active spike and its evidence matrix are recorded in [NOMS-016 clipboard import compatibility](../spikes/NOMS-016-clipboard-import-compatibility.md). Its first pass inspects provider markup and structured data; direct Chrome, Firefox, and Safari clipboard captures remain required before the compatibility cells can close.

Discovery should capture actual HTML and plain-text clipboard shapes from a deliberately varied set of recipe sites. Inclusion in this research list means only that the site exercises a useful page shape; it does not promise compatibility or override provider terms, authentication, access controls, or publication rights.

The primary spike set should be:

| Site | Page shape or behavior to exercise |
|---|---|
| Allrecipes | High-volume community recipes, ratings, scaling controls, and conventional ingredients/directions headings. |
| Food52 | Community and editorial recipes, author notes, named sections, and source/adaptation attribution. |
| Food Network | Media-publisher pages with named sub-recipes, grouped ingredients, and repeated yield/time metadata. |
| NYT Cooking | Subscription-aware pages using ingredients/preparation terminology and signed-in print behavior. |
| Serious Eats | Long-form editorial recipes, technique-heavy instructions, and complex component sections. |
| BBC Good Food | International formatting, metric quantities, and method terminology. |
| RecipeTin Eats | Independent high-traffic recipe cards with mixed metric and US customary presentation. |
| King Arthur Baking | Dual-unit baking quantities, ingredient alternatives, numbered instructions, and baker's notes. |
| Budget Bytes | Advertising-heavy blog layout with a compact recipe card and cost-oriented metadata. |
| Sally's Baking Addiction | Recipe-blog/plugin layout, baking notes, substitutions, and detailed timing. |
| Bon Appétit | Editorial layout, preparation terminology, access boundaries, and ingredient groupings. |
| Simply Recipes | Large publisher layout with descriptive content, structured recipe cards, and step imagery. |

If the primary set exposes materially different behavior that the generic parser cannot cover, the stretch set should include Tasty, America's Test Kitchen, Taste.com.au, The Kitchn, Minimalist Baker, Just One Cookbook, The Woks of Life, and Maangchi. These add video-first content, another subscription boundary, regional formatting, dietary specialization, and non-Western ingredient and section conventions.

For each investigated site, evaluate normal and print-friendly views where available, including:

- A simple recipe.
- Ingredient or instruction sections.
- A subordinate recipe or component such as a sauce.
- Missing or non-numeric time and yield values.
- Chrome, Firefox, and Safari behavior where those browsers are supported.

Repository tests must use minimal synthetic fixtures that reproduce the required document structure rather than retaining complete third-party recipe pages. The spike should record the observation date, browser, page/print mode, clipboard MIME types present, fields recovered, warnings produced, and whether a generic rule or source-specific refinement was required.

The generic parser should initially recognize `Ingredients` followed by `Directions`, `Instructions`, `Method`, or `Preparation`, then score nearby heading/list regions instead of parsing the whole page uniformly. The current evidence identifies Allrecipes and RecipeTin Eats as likely adapter candidates. Direct Serious Eats, NYT Cooking, and Bon Appétit captures should first exercise the generic parser plus narrowly specified cleanup; they become adapters only if a synthetic fixture demonstrates a correctness failure and a stable provider rule resolves it. The same fixture-first approval rule applies to every adapter.

The first direct provider result establishes an Allrecipes-specific recommendation: use its `Print` action, select all, and copy. That shape retains the complete core recipe and attribution while removing most editorial and control noise, and it should use the generic parser after sanitization. An Allrecipes adapter remains necessary only for normal-page or whole-page fallback. Provider-specific guidance must not imply that a copied print view supplies the canonical source URL.

For the captured RecipeTin Eats page, the print workflow is provisionally preferred over whole-page copying: the normal-page payload was approximately five times larger in HTML and 4.2 times larger in text and began with navigation, controls, repeated titles, and long-form editorial content. The captured print shape retains print/scaling controls and both abbreviated and full procedures. Before sanitization discards styles, state classes, data attributes, or controls, a narrow RecipeTin cleanup rule must resolve any reliably indicated active unit/yield variant into neutral semantic markers. It then preserves grouped ingredients and notes, prefers a non-empty, structurally valid `FULL RECIPE`, falls back to a valid `ABBREVIATED RECIPE` when necessary, and warns whenever it discards a competing procedure. The normal-page fallback applies these refinements only when it detects the corresponding structures. Both paths feed the same generic parser rather than introducing an independent recipe parser. Print becomes provider-wide guidance only after additional-recipe and named-browser captures plus a synthetic parser fixture confirm the behavior; normal-page copying remains an accepted WPRM-card fallback.

The captured Bon Appétit page has no dedicated print action. Whole-page copying is the provisional primary workflow because it is simple and retains title, author, date, rating, description, and image attribution, although it produces approximately 423,000 UTF-16 code units of styled HTML; this recommendation remains subject to the payload-limit and parser-performance fixture checks. The proposed cleanup contract locates the later `Recipe information` → `Ingredients` → `Preparation` card, reconstructs adjacent quantity and unit/name nodes, preserves unitless ingredients when the amount node is empty or absent, removes meal-plan/`Ask AI` and merchandising controls, and ends the primary procedure before an in-list `VARIATIONS` block. It becomes a narrow Bon Appétit adapter only if a synthetic fixture proves the generic parser insufficient. A directly captured card-focused selection reduced HTML by approximately 87% and text by 43%, so it is the recovery workflow for oversized or ambiguous imports; because it omits title, attribution, description, and image metadata, the preview must request manual completion of those fields. Provider-wide guidance still requires additional recipes, access states, named browsers, and a synthetic parser fixture.

For the captured NYT Cooking page, logged-out whole-page copying is the provisional anonymous workflow candidate. It produced both HTML and plain text; the HTML retained recipe-region signals, inline structure, images, and links but no scripts or embedded Schema.org JSON-LD, while the clipboard selection retained recipe/adaptation attribution. The full payload was not persisted, so the workflow is confirmed only after a synthetic fixture proves complete ingredient and preparation recovery from the observed shape. The logged-out `Print Recipe` action redirected to the New York Times sign-in flow, so print copying must not be required and Noms must not bypass the access boundary. The sanitized HTML should first use the generic `Ingredients` → `Preparation` scorer, preserving attribution and warning when the plain-text fallback recovers only introductory metadata. A narrow NYT adapter is allowed only if the fixture proves the generic parser insufficient. Signed-in print behavior, additional recipes and access states, and named browsers remain verification work rather than prerequisites for evaluating the anonymous workflow.

For the captured Serious Eats page, the site's `?print` route is provisionally preferred over normal-page whole-page copying. It reduced HTML by about 88% and plain text by about 81%, began directly with provider/title/description/attribution/times/yield/ingredients, and retained direction signals while omitting images. The workflow is `Print` → dismiss the browser print surface when necessary → select all → copy. The sanitized print shape should first use the generic headings-and-lists parser and report the unavailable image as a warning, not a failed import. Normal-page copying remains a fallback that must locate the later recipe card after navigation and long-form editorial content and remove article figures and captions. Because the inspector persisted only bounded previews, complete ingredient and instruction recovery and the print-surface interaction must be confirmed by a synthetic fixture and named-browser captures before this becomes provider-wide guidance. A Serious Eats adapter is allowed only if that fixture proves narrow generic boundary cleanup insufficient.

For the captured BBC Good Food page, normal-page whole-page copying is the provisional workflow because no print-labelled control was exposed, subject to payload-limit and parser-performance checks for its approximately 358,000 UTF-16 code units of HTML. The clipboard retained title, author, yield, difficulty, times, rating, description, labels, ingredient and method signals, images, and links, alongside navigation, ads, app/cook-mode prompts, media status, and nutrition/unit controls. The generic `Ingredients` → `Method` scorer should remove that surrounding noise, preserve ingredient continuation lines for preview, and retain only a reliably active unit variant before sanitization discards state markers; it must never merge metric and alternate units. When active state cannot be established, the parser preserves the variants as competing candidates and warns instead of choosing or discarding one. A BBC Good Food adapter is allowed only if a synthetic fixture proves the generic cleanup insufficient. Correctness, payload-size and performance checks, additional recipes, continuation notes, unit variants, and named-browser captures remain required before provider-wide guidance is confirmed.

For the captured King Arthur Baking page in a logged-out/public state, the site's `Print` action is provisionally preferred over normal-page whole-page copying. Selecting all and copying from the selectable provider print document exposed by the browser print surface reduced HTML by about 91% and plain text by about 64%, began directly with provider/title/description/times/yield/image description/ingredients, and retained direction signals, image elements or references, and links. Its copied HTML exposed the provider path `/recipes/print/16646`, but this is a provider-detection hint rather than the canonical recipe URL; the preview still needs the optional source-URL field. The supported workflow uses the visible `Print` action rather than constructing or fetching print URLs, and falls back to normal-page copying when a browser exposes only non-selectable native print UI. The generic headings-and-lists parser should preserve paired volume and parenthesized metric quantities as one ingredient representation, retain only safe image references, alternative text, and credits, and never fetch remote media during parsing. The normal-page fallback removes navigation, promotions, controls, and repeated image descriptions before scoring the recipe region. Provider-specific cleanup is allowed only if a synthetic fixture proves a stable boundary or duplication failure. Complete recovery, additional recipes, dual-unit cases, print-surface behavior, and named-browser captures remain required before provider-wide guidance is confirmed.

For the captured Budget Bytes page in a logged-out/public state, the dedicated route linked by `Print Recipe` is provisionally preferred over normal-page whole-page copying. It reduced HTML by about 84% and plain text by about 78%, began with print/unit/yield controls followed by rating/title/description/times/metadata/equipment/ingredients, and retained direction signals while omitting image elements. The user workflow uses the visible print link followed by select-all and copy; the importer must not construct or fetch the route itself. The `/wprm_print/...` path is only a provider-detection hint, not the canonical recipe source, so the preview still needs the optional source-URL field. Before sanitization removes state markers, the parser should retain only a reliably active unit and yield representation, or preserve competing candidates and warn when state is uncertain. It should preserve grouped ingredients, instruction section headings, equipment, and notes for preview. Per-ingredient prices are optional provider annotations: separate them into import-only metadata only with deterministic rules, otherwise keep the raw line for review. Normal-page copying remains a fallback that scores the recipe card and excludes editorial content, controls, reviews, nutrition UI, and the later illustrated procedure duplicate. A narrow Budget Bytes cleanup is allowed only if a synthetic fixture proves stable state, price, or boundary rules are required. Complete recovery, additional recipes, active unit/yield cases, named browsers, and print-link behavior remain verification work.

For the captured Sally's Baking Addiction page in a logged-out/public state, the dedicated route linked by `Print Recipe` is provisionally preferred over normal-page whole-page copying. It reduced HTML by about 84% and plain text by about 76%, began with an image description followed by title/rating/author/times/yield/description/ingredients, and retained direction signals, image elements or references, and links. Its print representation indicated images, description, and notes were enabled, though complete ordered ingredient, instruction, and notes recovery remains fixture work. The user workflow uses the visible print link followed by select-all and copy; Noms must not construct or fetch the route. The print path is only a provider-detection hint, not the canonical recipe source, so the optional source-URL field remains necessary. The generic parser should preserve paired volume/weight measurements, internal alternatives, raw multi-option yield text, footnote markers and associated notes, and safe image references/alternative text/credits without fetching remote media. It must not normalize the observed alternative yield as a numeric range, and it must retain an unmatched footnote marker in the raw ingredient while warning in the preview. Normal-page copying remains a fallback that scores the later recipe card and excludes promotions, affiliate/tool links, editorial ingredient lists, success tips, FAQs, testimonials, and other numbered prose. A narrow Sally's cleanup is allowed only if a synthetic fixture proves stable boundary or footnote rules are required. Complete recovery, additional recipes, footnote/notes cases, print-link behavior, and named browsers remain verification work.

For the captured Simply Recipes page in a logged-out/public state, normal-page full-page copying is the current workflow candidate. The visible `PRINT` action opened browser print UI without a provider-owned print URL; the user performed select-all and copy while it was shown, and the resulting capture had identical plain-text length, only 49 fewer HTML code units out of roughly 466,000, matching bounded previews, and matching recorded structure signals. Because complete payloads were not persisted and it is unclear whether native print UI or the underlying page supplied the selection, this establishes only that no distinct or materially cleaner print representation was observed. Print copying should not be recommended unless named-browser evidence shows such a representation. The normal payload remains subject to payload-size and parser-performance gates. The generic scorer should locate the later `Ingredients` → `Method` → `Recipe Notes` region, preserve author/update metadata and safe image references/alternative text/credits for preview without fetching remote media, and remove navigation, app promotion, controls, editorial/table-of-contents content, nutrition UI, reviews, and unrelated images. Step images and repeated photographer credits must not become instruction text; discarded media metadata requires a warning. Before full-page copying becomes provider-wide guidance, compare it with a focused selection from the recipe-card title through `Recipe Notes`. A narrow Simply Recipes cleanup is allowed only if a synthetic fixture proves stable media-credit or boundary rules are required. Complete ordered recovery, focused-selection comparison, payload/performance checks, additional recipes, media-credit cases, print behavior, and named browsers remain verification work.

For the captured Food52 page in a logged-out/public state, normal-page full-page copying is the current workflow candidate. It exposed both HTML and plain text, retaining title, rating, author, image credit, times, yield, description, ingredients, and directions after navigation and cart noise, but produced approximately 917,000 UTF-16 code units of heavily styled HTML. The visible `Print` action opened browser print UI without exposing a provider-owned print URL; copying while it was shown produced essentially the same bounded text and structure signals, one additional plain-text code unit, and approximately 30,000 additional HTML code units. Because complete payloads were not persisted and it is unclear whether native print UI or the underlying page supplied the selection, this establishes only that no distinct or materially cleaner print representation was observed. Print copying should not be recommended unless named-browser evidence shows one. The generic scorer should locate the title/metadata and `Ingredients` → `Directions` region; preserve attribution, displayed rating metadata, and safe media metadata without fetching remote resources; remove interactive rating controls with other navigation, cart, video/promotional, and unrelated content; and reconstruct quantity/unit/name fragments from DOM boundaries only when a synthetic fixture proves that rule deterministic. When HTML boundaries are absent or insufficient, the plain-text fallback must preserve the collapsed raw ingredient line and warn rather than guess inserted spaces. Before full-page copying becomes provider-wide guidance, compare it with a focused title-through-final-direction selection. A narrow Food52 cleanup is allowed only if a synthetic fixture proves stable fragment-joining or boundary rules are required. Complete ordered recovery, focused-selection comparison, payload/performance checks, additional recipes, grouped-section/video cases, print behavior, and named browsers remain verification work.

For the captured Food Network page in a logged-out/public state, normal-page full-page copying is the current workflow candidate. It exposed both HTML and plain text, with approximately 228,000 UTF-16 code units of HTML and 5,077 of text, and retained attribution, title, displayed rating/review count, difficulty, raw times, yield, description, ingredients, directions, and a `Cook’s Note` after navigation noise. The visible `Print Recipe` action opened browser print UI without changing to a provider-owned URL; copying while it was shown produced identical plain-text length, matching bounded previews and structure signals, and 1,583 additional HTML code units. Because complete payloads were not persisted and it is unclear whether native print UI or the underlying page supplied the selection, this establishes only that no distinct or materially cleaner print representation was observed. Print copying should not be recommended unless named-browser evidence shows one. The generic scorer should locate the attribution/title/metadata plus `Ingredients` → `Directions` → `Cook’s Note` region; preserve ordered directions even when one step contains multiple sentences or paragraphs; remove ingredient checkboxes, `Deselect All`, shopping-list and other interactive controls; and exclude navigation, reviews, recommendations, private-note UI, and unrelated media. The observed `Active: 20 hr` conflicts with `Total: 1 hr 30 min`; Noms must retain that raw value, warn that the duration is suspicious, and never silently reinterpret its unit. Before full-page copying becomes provider-wide guidance, compare it with a focused attribution-through-`Cook’s Note` selection. A narrow Food Network cleanup is allowed only if a synthetic fixture proves stable control-removal, note-boundary, or multi-paragraph-step rules are required. Complete ordered recovery, focused-selection comparison, additional recipes, suspicious-duration and multi-paragraph-step cases, print behavior, and named browsers remain verification work.

The first-pass direct clipboard matrix now contains at least one in-app-browser observation for all twelve primary providers. It establishes whole-page `Ctrl+A`/`Ctrl+C` as the provider-neutral, best-effort first attempt and is sufficient to begin implementation. This is evidence for parser and fixture design, not a compatibility-release claim: synthetic fixtures and parser-prototype results will determine actual provider feasibility during implementation. Focused selections, print views, additional recipes and access states, and named Chrome/Firefox/Safari captures remain recovery and release-verification work rather than prerequisites for starting the implementation.

The spike proposes Chrome as the first release-gate browser for the clipboard workflow. Firefox and Safari must also be captured and tested; format differences may use a documented plain-text fallback only when all required visible ingredient and instruction content remains recoverable and the preview communicates lost structure. The browser release policy requires approval after the captures are compared.

### Clipboard trust boundary

Pasted HTML is untrusted. The implementation must:

- Neutralize resource-bearing attributes before DOM insertion, or use a proven non-fetching HTML parser; a detached document alone is not treated as a no-network guarantee.
- Never render unsanitized HTML.
- Remove scripts, styles, frames, forms, event handlers, and executable or active content.
- Avoid loading remote images or other resources during parsing.
- Enforce input-size and structural-complexity limits.
- Validate and normalize extracted URLs.
- Retain the normalized candidate rather than the complete pasted page.
- Require explicit confirmation before saving and warn before public publication of imported content.

NLP extraction, a browser extension or side-panel clipper, and a native embedded browser are follow-up options, not requirements for this initial milestone. The clipboard parser and preview contract should nevertheless be reusable by those future entry points.

## Export contract

Noms exports portable recipe snapshots rather than complete database backups. Ordinary exports must not include drafts, complete version history, diffs, fork messages, or internal operational records. The supported export scopes are:

- One current recipe snapshot.
- One explicitly selected historical snapshot when exporting from a version view.
- One collection subtree containing the selected collection, its nested collections, and their recipe snapshots.
- All collection trees, excluding uncollected recipes unless the user explicitly selects an entire-library scope in a future release.
- An optional, deliberately selected lineage export containing one current or selected snapshot per lineage member and only the parent-child edges needed to display or reconstruct the graph.

### Canonical Noms recipe snapshot

`noms.recipe.snapshot.v1` is the authoritative lossless interchange contract. It must contain every field in the portable recipe model, including fields whose values are `null`, empty strings, empty arrays, or empty objects. NOMS-017 must define the portable taxonomy fields before this schema is frozen.

The canonical snapshot must distinguish three field states:

- **None:** the canonical value is explicitly `null`; no value was recorded.
- **Empty:** the field is present but contains an empty string, list, or object.
- **Missing:** a value or asset was expected but was unavailable, could not be recovered, or could not be represented by the selected format.

Every canonical recipe field must be present, so `Missing` is not represented by silently omitting a key. If a required snapshot field cannot be read, canonical snapshot generation fails. `Missing` applies to renderer limitations and unavailable referenced assets and is recorded in export-envelope field-status metadata alongside the still-complete canonical snapshot. Machine-readable Noms JSON must preserve the actual JSON values (`null`, `""`, `[]`, or `{}`) and must not replace them with display strings.

Presentation renderers must visibly label every absent field as `None`, every empty field as `Empty`, and every unavailable or unrepresentable field as `Missing`, optionally followed by a reason such as `Missing — excluded by image mode`. A format that conventionally omits absent values, such as Schema.org JSON-LD, must record the field status in Noms export metadata or a companion sidecar rather than emitting invalid or misleading standard properties.

Every export envelope must include the portable format and version, generator name and version, export timestamp, scope, portable ID, snapshot digest, source attribution, image mode, and any fields the selected renderer could not represent. Export-envelope metadata is not part of the recipe snapshot itself.

### Portable identity and duplicate detection

Original Noms recipes export a stable portable identity of the form `urn:noms:recipe:{recipe_uuid}`. A re-imported recipe receives a new local database UUID while retaining its origin portable ID as provenance. Portable identity is not proof of ownership and must not be used for authorization.

Re-import behavior is:

- An unknown portable ID imports as a new private draft.
- A known portable ID with an identical snapshot digest is reported as a duplicate and skipped by default.
- A known portable ID with a different snapshot digest is reported as a conflict and may be imported as a copy; it must never overwrite silently.
- A matching title with a different portable ID is treated as a distinct recipe.

### Snapshot digest

The snapshot digest identifies the exact logical content independently of whitespace, object-key order, export time, output filename, renderer, or image-packaging choice. Noms must:

1. Build the complete `noms.recipe.snapshot.v1` value with explicit null and empty fields.
2. Normalize set-like fields such as tags by applying the schema's documented deduplication and ordering rules; preserve order for ingredients, instructions, equipment, images, and every other order-sensitive array.
3. Canonicalize the snapshot using the JSON Canonicalization Scheme (JCS) from [RFC 8785](https://www.rfc-editor.org/info/rfc8785/).
4. Encode the canonical representation as UTF-8.
5. Calculate SHA-256 and record the algorithm, canonicalization method, snapshot-schema identifier, and digest value.

The digest input excludes the portable ID, export-envelope metadata, export timestamp, generator version, collection membership, lineage edges, archive paths, and the choice to package or omit image bytes. Logical image records remain part of the snapshot; packaged image files receive separate SHA-256 checksums in archive metadata. A future snapshot-schema change must use a new schema identifier so old and new digest meanings cannot be confused.

### Export formats and phases

All renderers consume the same canonical snapshot and declare any field they cannot represent. Formats are implemented in this order:

1. **Phase 1 — core portability:** versioned Noms JSON, Schema.org `Recipe` JSON-LD, and offline HTML with equivalent embedded JSON-LD.
2. **Phase 2 — common presentation:** Markdown, PDF, and plain text.
3. **Phase 3 — additional interoperability:** YAML, Cooklang, and verified third-party formats such as Paprika when their contracts and compatibility expectations have been validated.

Noms JSON is the authoritative re-import format. Schema.org JSON-LD preserves the standard recipe subset. HTML must be viewable offline and printable. PDF is presentation-only and is not a canonical interchange format. CSV is not a required recipe format because it cannot represent nested recipe data faithfully.

Every presentation format must include all recipe fields plus export-specific metadata such as portable ID, digest, export time, format version, generator version, source attribution, and image mode. When a field does not have a natural primary-layout representation, the renderer must include it in a clearly labeled metadata or details section rather than silently dropping it. Renderer tests must verify `None`, `Empty`, and `Missing` output explicitly.

### Collection archive and re-import

Collection archives mirror the existing nested collection hierarchy as folders. Each collection folder contains a `_collection.json` descriptor plus child collection folders and recipe folders. Folder names may use sanitized, ordered display names, but portable identity and metadata must never depend solely on filesystem names.

The `_collection.json` descriptor preserves the original collection name, description, image, visibility, sibling position, recipe ordering, child ordering, format version, and any explicit null or empty fields. It also maps each recipe portable ID to its folder. This descriptor is required so empty collections, unsafe filename characters, ordering, and metadata survive a round trip.

Each recipe folder contains canonical `recipe.json`, the presentation files selected by the user, and an optional `images/` directory. A recipe present in multiple collections may be represented in each corresponding folder for a natural offline layout; on re-import, matching portable IDs and digests must resolve to one imported local recipe that is attached to every applicable collection.

Collection import reconstructs parent-child collections from directory nesting, restores descriptor metadata and ordering, imports recipe files as private drafts, applies the normal duplicate/conflict rules, and validates every archive path against traversal and filename attacks.

### Simple lineage export

A lineage archive contains one current or explicitly selected snapshot per included member and a small `lineage.json` descriptor. The descriptor contains portable member IDs and parent-child `derived_from` edges only. It excludes drafts, complete version histories, diffs, fork messages, and unrelated ownership or operational data.

### Image export modes

The export UI must offer explicit image modes:

- `none`: omit packaged files and image references from presentation output while retaining canonical field status.
- `references`: retain URLs and attribution without copying bytes.
- `stored`: package image bytes already stored and viewable through Noms.
- `all`: package stored images and explicitly attempt to retrieve referenced remote images.

Any image attached to an exportable snapshot may be selected; exporting is not restricted to images uploaded by the exporting user. Every image must still pass the same view authorization as its recipe. Remote images are never fetched implicitly. The `all` mode must preserve original URL and attribution, apply public HTTP(S), DNS/redirect, SSRF, content-type, decompression, file-size, and timeout protections, and report individual retrieval failures as `Missing` without failing the entire recipe export. SVG requires rejection or a separately specified sanitization path.

Packaged images receive stable archive paths and SHA-256 checksums. The image mode does not alter the logical snapshot digest. Formats that cannot embed bytes may emit references and record the limitation explicitly.

### Client-orchestrated generation

The first release keeps server APIs small and has the Dioxus client orchestrate multi-recipe export generation. The server exposes authorized operations to retrieve an immutable recipe export snapshot, retrieve a collection-tree descriptor, and retrieve or safely fetch an allowed image. Each operation rechecks the same visibility rules used by the corresponding recipe, version, collection, lineage, or image view.

The client:

1. Chooses one export start timestamp and options shared by the whole archive.
2. Retrieves the authorized collection tree or lineage descriptor.
3. Fetches recipe snapshots and selected assets with bounded concurrency.
4. Applies the requested renderers to the common snapshot model.
5. Builds the archive in browser memory, reports progress, supports cancellation, and triggers the download.

Client orchestration shifts memory use away from the server but does not eliminate it. Before generation, the client must preflight recipe, collection, image, and estimated-byte counts. Both client and server enforce configured per-item and total limits; the client also enforces actual archive-size and generation-time limits. An oversized export must stop with a clear request to reduce scope or disable packaged images. Streaming and background jobs remain future options.

Collection generation is not transactionally frozen across multiple API calls. Each returned recipe is an immutable selected snapshot with its own digest, and the archive records the overall start time plus per-snapshot retrieval metadata. Bounded concurrency and retry rules must prevent the client from creating an accidental request burst.

### Export authorization and privacy

A user may export a recipe snapshot only when that user is authorized to view the same snapshot through Noms. The rule applies independently and recursively to every recipe, selected version, nested collection, lineage member, and packaged image. Authorization must not be inferred from portable identity or archive paths.

The server returns only portable, user-visible snapshot fields. Exports must not contain credentials, sessions, OAuth data, internal storage credentials, signed storage URLs, private database ownership details, or operational metadata. Source and author attribution remain present. A collection or lineage export must fail clearly rather than silently producing a partial archive when a required member is no longer viewable; remote image retrieval failure is the exception and is represented as `Missing`.

### Export verification matrix

Automated and manual verification must cover:

- Exact Noms JSON round trips with explicit null and empty field states, plus missing renderer/asset status metadata.
- Stable JCS/SHA-256 digests across whitespace and object-key ordering changes, and changed digests for logical snapshot changes.
- Portable-ID duplicate and conflict handling.
- Nested instructions, sections, Unicode, fractions, metadata, and ordered fields.
- Every renderer's declared fidelity and visible `None`, `Empty`, and `Missing` representations.
- Schema.org validation and offline HTML rendering/printing.
- Image modes, checksums, missing assets, unsafe remote responses, and archive-size limits.
- Nested and empty collections, unsafe names, ordering, recipes in multiple collections, and collection re-import.
- Simple lineage graph export and reconstruction.
- Recursive authorization and prevention of partial unauthorized archives.
- Client progress, cancellation, bounded concurrency, preflight rejection, and cleanup after errors.

## Questions to resolve

- Which copied-page shapes must the generic clipboard parser support before source-specific adapters are added?
- Which target browsers and provider page/print layouts form the first compatibility matrix?
- What confidence and warning model is exposed by parsers to the preview UI?
- What input-size and structural-complexity limits apply to pasted HTML and text?
- When URL fetching is added, what rate limits, SSRF protections, provider-policy rules, and failure behavior apply?
- What concrete client/server item, byte, concurrency, and time limits apply to in-memory export generation?
- Which Rust/WASM-compatible archive, PDF, HTML-sanitization, JCS, and hashing libraries satisfy the export contract?

## Required implementation-plan deliverables

- Current-state code and schema baseline.
- Import and export contracts with explicit trust boundaries.
- Clipboard compatibility fixtures and deterministic parsing rules.
- Versioned snapshot, portable-ID, digest, collection, lineage, and renderer contracts.
- Test-first checkpoints, migrations, UI states, and rollback considerations.
- Compatibility rules for NOMS-017 metadata.
- Automated, manual, security, and portability verification matrix.

## Definition of ready

- [ ] Product scope and supported formats are approved.
- [ ] Clipboard compatibility targets, sanitization, and resource limits are specified.
- [ ] The canonical portable recipe schema, portable identity, digest, and missing-field semantics are documented.
- [ ] Collection, lineage, image, renderer, and client-orchestration contracts are documented.
- [ ] Checkpoints, acceptance evidence, and release gates are defined.

## Spike progress (2026-09-01)

- The private/local clipboard corpus contains 1,500 paired HTML and plain-text samples across 15 providers; only aggregate reports and invented fixtures are committed.
- The import-candidate v1 contract and eleven synthetic fixtures are executable. The cumulative region-quality slice covers clean HTML, plain text, grouped components, competing regions, embedded collection recipes, navigation false positives, and missing titles.
- The generic parser's region-quality rerun structurally accepted 719 of 1,500 samples with no invalid candidates or unexpected errors. All accepted candidates have titles; the 80 observed Food Network false positives are no longer accepted, and the two reviewed Sally's collection cases require confirmation.
- The unit/yield/time slice parses labeled durations and selected yield controls, including externally associated and accessible labels. It handles visible measurement variants while preserving inactive or ambiguous alternatives and contradictory times for review.
- The notes/equipment/image slice keeps supplementary sections separate from ingredients and instructions, preserves absolute HTTP(S) figure references and captions without fetching them, and requires review before notes or images are mapped.
- The active-content slice makes script, frame, control, and unretained remote-reference removal explicit while keeping script, style, iframe, template, and noscript bodies opaque, including malformed quoted or unclosed opening tags. Audit evidence survives normal extraction, unsupported-candidate returns, and HTML-to-plain-text fallback. The unsupported access-shell case is the final generic parser slice. After that rule, provider adapters or Schema.org JSON-LD extraction should target stable gaps rather than broad heuristics.
- The digest-bound generic-parser baseline reviewed 27 accepted real-corpus candidates: 5 usable without correction, 20 usable after correction, and 2 unusable. Core fields were correct in 73 of 108 reviews (67.6%): titles were strongest, while structured ingredient parsing and yield recovery were the clearest priorities. This purposive sample diagnoses failure modes and is not a provider compatibility estimate.
- The first exact-host RecipeTinEats adapter raised that provider's structural acceptance from 19/100 to 81/100. A follow-up rule ignores structurally empty WPRM instruction placeholders only when at least one complete group remains, raising acceptance to 98/100 without invalid candidates or unexpected errors. The two remaining samples proved to be editorial pages rather than individual recipes and were replaced with genuine recipe captures; all 100 applicable RecipeTinEats samples now parse. Re-reviewing the same three RecipeTinEats candidates raises the overall semantic benchmark to 79/108 correct core fields (73.1%), with 6 usable without correction, 20 after correction, and 1 unusable. The adapter activates only for one complete inert WPRM card and otherwise falls back to generic parsing.
- The exact-host Tasty adapter keeps structural acceptance at 100/100 while repairing measured completeness gaps: it retains 1,265 ingredient rows across all 192 visible sections instead of the generic parser's 773 rows, and returns 743 recipe steps after excluding 99 exact app-download prompts. All 100 candidates include title, description, servings, total time, and one deduplicated image; 86 include labeled prep and cook times, and 99 include an author. All candidates validate, no non-Tasty structural result changed, and ambiguous or incomplete provider markup still falls back to generic parsing. A narrow follow-up rule preserves compound two-unit measurements as one amount expression; it matched three Tasty captures. Re-reviewing the three Tasty candidates now gives that provider 12/12 correct core fields and raises the overall benchmark to 85/108 (78.7%), with 9 candidates usable without correction, 18 after correction, no stale labels, and no unusable candidates.

Evidence and reproduction commands are recorded in [the real-corpus parser evaluation](../spikes/NOMS-016-real-corpus-parser-evaluation.md), [the semantic parser evaluation](../spikes/NOMS-016-semantic-parser-evaluation.md), and [the import-candidate contract](../spikes/NOMS-016-import-candidate-contract.md).

## Provider-adapter rollout

Provider adapters share the inert clipboard normalization and candidate contract with the generic parser. An adapter must use an exact supplied hostname plus stable visible structure, must never fetch or execute content, and may activate only when it can establish one complete recipe. Adapter-owned core fields may be reconciled with safe supplementary evidence from the shared parser, such as image references and attribution, so stronger structural extraction does not discard independently reviewable evidence. Missing, changed, or ambiguous provider structure falls back to generic parsing and review rather than producing a partial authoritative result.

Each adapter requires synthetic exact-host, lookalike-host, complete-field, ambiguity, and fallback tests; a contract-valid full-provider corpus rerun; refreshed digest-bound semantic labels; and evidence that generic-provider results did not regress. Prefer data-driven selectors and shared extractors over provider code where structures are genuinely equivalent.

RecipeTinEats, Tasty and BudgetBytes are completed, corpus-verified adapters with current semantic reviews. BudgetBytes improves from 65/100 to 100/100 structurally accepted captures, retaining 1,297 ingredients across 159 sections and 764 steps. It preserves explicit parenthesized conversions, cleans narrow price annotations from parsed names while retaining original text, and recovers card metadata, notes and image references. Its three reviewed candidates improve from 6/12 to 12/12 correct core fields. The overall semantic benchmark is now 91/108 (84.3%), with 12 usable without correction, 15 after correction and none unusable. The full corpus accepts 835/1,500 candidates, and all 1,400 non-BudgetBytes outputs are unchanged.

AllRecipes was selected next following a successful user check of the Tasty UI. Its adapter is implemented and verified on 100/100 saved captures, retaining 808 ingredients in 103 sections and 564 clean steps. It recovers structured quantities, metadata, recipe tips/cook's notes and unfetched image references while conservatively rejecting incomplete, hidden or ambiguous adapter structures. All 1,400 other-provider outputs are unchanged. The three AllRecipes benchmark recipes improve to 12/12 correct core fields; the overall benchmark is now 93/108 (86.1%), with 14 usable without correction, 13 after correction and none unusable. Structural acceptance remains 835/1,500. The refreshed review is stored separately at `/home/samko/Documents/Recipe Data/semantic-review-allrecipes-v1.json`; the original external review is preserved.

Food52 followed the user's successful AllRecipes UI check. Its exact-host visible-article adapter is implemented and verified on all 100 saved captures, raising Food52 structural acceptance from 83/100 to 100/100 and the full corpus to 852/1,500. It retains 1,423 ingredient rows across 136 groups and 619 steps across 117 direction groups, plus author, description, labeled times/yield and unfetched hero references. The 1,400 non-Food52 outputs are unchanged. Unmarked ingredient rows remain low-confidence with a review warning rather than guessed section labels; one capture's empty group headings are retained as unnamed groups with a warning. Missing, hidden, conflicting or unaccounted core structures fall back to generic parsing.

The three Food52 benchmark candidates now score 10/12 correct core fields, with two usable without correction and one requiring correction of unmarked section labels. The overall benchmark is 94/108 (87.0%), with 15 usable without correction, 12 after correction and none unusable. The current digest-bound review is `/home/samko/Documents/Recipe Data/semantic-review-food52-v1.json`. The browser confirms full clipboard paste → preview → private draft → autosave/reload on port 32773; images remain references only and group labels are preserved in optional commentary.

Food.com followed the user's successful Food52 check. Its exact-host adapter now accepts all 100 saved captures, up from 90/100, retaining 869 ingredient rows across 110 groups and 656 steps across 104 groups. It preserves marked fractions, ranges, package units, explicit section headings, descriptions, authors, total time, literal yield and unfetched hero references. Servings are inferred only from a positive integer `Serves` value, never from an output count or range; `Ready In` remains total time rather than guessed prep/cook time. Hidden, competing, malformed or unaccounted core structures refuse the adapter and retain generic fallback. All 1,400 other-provider outputs are unchanged; full structural acceptance is now 862/1,500.

The three Food.com benchmark candidates improve from 8/12 to 12/12 correct core fields and are usable without correction. The overall purposive benchmark is 98/108 (90.7%), with 18 usable without correction, 9 after correction and none unusable. The current external digest-bound review is `/home/samko/Documents/Recipe Data/semantic-review-foodcom-v1.json`; previous reviews are preserved. A browser smoke test confirms full HTML/text clipboard paste → preview → private draft → autosave/reload for Italian Bread on port 32773, retaining 10 ingredients, 31 steps, six servings and original time/yield commentary. Images remain references only. These measurements do not establish semantic accuracy for every saved capture or rank the remaining providers.

The user's subsequent Food.com check exposed appended recipe roots displacing the primary recipe under generic fallback. A follow-up now scopes extraction to the unique `main#recipe` with disjoint, unmarked recipe siblings, warns that appended recipes were ignored, and removes recognized description expand/collapse labels. Ambiguous, nested or malformed roots remain conservative fallbacks. Eighteen Food.com tests pass; all 100 saved captures still parse, other-provider outputs remain identical, and the 27-sample benchmark remains digest-current. The fresh NOMS-016 UI at port 32783 previews the reported chicken recipe with its description using DOM-derived HTML/text session clipboard input; native OS copy fidelity was not verified. Details and evidence are in the real-corpus report.

After the user verified the Food.com follow-up, NYT Cooking was implemented next. Its exact-host visible-component adapter accepts all 100 saved captures, retaining 1,098 ingredients across 103 groups, 435 complete steps and 17 recipe tips. Description paragraphs, author/adaptation attribution, literal yield, labeled durations and unfetched hero references are recovered without importing update timestamps, reader notes or step-media credits. Compound measurements remain literal in one amount field; ranged/output yields never become guessed numeric servings. Qualified durations remain raw text with a null numeric value and a review warning. Missing, hidden, unmatched, competing or unconsumed core markup refuses the adapter.

Eighteen synthetic NYT tests pass, including independent-review regressions for unmatched roots and marked required content inside ignored controls/media. All 1,400 non-NYT outputs are unchanged; full structural acceptance remains 862/1,500. The three re-reviewed NYT candidates now have 12/12 correct core fields and are usable without correction of the reviewed fields. The purposive benchmark is 100/108 (92.6%), with 20 usable without correction and 7 after correction. Current NYT review/artifacts are temporarily `/tmp/noms016-nyt-review.json` and `/tmp/noms016-nyt-semantic.{json,md}`; previous external reviews remain intact.

A synthetic-only browser smoke test on port 32783 confirms full HTML/text session clipboard paste → preview → private draft → reload, preserving structured quantities, metadata and optional commentary. The existing canonical instance was restarted and reloaded without cached assets after an initial stale-parser result; the one-instance-per-worktree guard prevented creating a concurrent fresh instance. This proves the current deployed adapter/mapping flow, not native OS clipboard fidelity or live NYT access. NYT Cooking is ready for the user's manual UI check. Subsequent adapters should target the remaining reproducible provider gaps with the same synthetic-test, full-corpus and semantic-review gates.

After the user verified NYT Cooking, Serious Eats was implemented next. Its exact-host adapter accepts all 100 saved captures (previously 98), retaining all 1,300 ingredient rows and 571 complete steps. Ingredient conversions stay literal; photo credits, ads, related links and comments never become directions. Explicit grouped ingredients, day-by-day phases, one nested single-item procedure, equipment and paragraph/list notes are supported. Conflicting total-duration aliases retain both raw labels with null numeric values and a review warning. Qualified durations and ranged/output yields remain literal. Hidden, malformed, competing or unconsumed required markup refuses the adapter; hidden lead images are excluded and all image references remain unfetched.

Twenty-one synthetic Serious Eats tests and the independent review pass. All 1,400 other-provider outputs remain identical; integrity-verified full structural acceptance is 864/1,500. The three re-reviewed Serious Eats candidates reach 12/12 correct core fields and are usable without correction of the reviewed fields. The purposive benchmark is 102/108 (94.4%), with 22 usable without correction and 5 after correction. Current temporary review/reports are `/tmp/noms016-serious-review.json` and `/tmp/noms016-serious-semantic.{json,md}`. The browser confirms invented full HTML/text session clipboard paste → preview → private draft → title autosave → reload on port 32783, plus rejection of an access shell. No live provider access or native OS clipboard fidelity was verified. Serious Eats is ready for the user's manual review. Details and commands are in the real-corpus report.

After the user's Serious Eats verification, Sally's Baking Addiction was implemented next. Its exact-host visible-card adapter accepts all 100 saved captures, up from 64 generic acceptances. Source accounting retains all 1,260 ingredient rows across 175 groups, 808 complete steps and 656 notes. Written mixed fractions, conversions, package-size units and compound measures survive without consulting scaling attributes; qualified durations and output yields remain literal. Leaf description wrappers and structurally empty editor placeholders are supported narrowly. Required text outside recognized section bodies/headings, hidden or unmatched markers and competing cards refuse the adapter. A surrounding page-title mismatch exposes competing evidence for confirmation; images remain safe unfetched references.

Twenty-one synthetic tests and independent review pass, including the required-section omission regression found during review. All 1,400 other-provider outcomes remain identical to `190d46d`. Full integrity-verified structural acceptance is 900/1,500, with zero invalid candidates or unexpected errors. The three Sally's benchmark candidates now have 12/12 correct core fields; the purposive full benchmark is 104/108 (96.3%), with 24 usable without correction and 3 after correction. This is an implementation-agent source-to-output review, not provider-wide accuracy. Browser session clipboard paste → preview → private draft → title autosave → reload succeeds on port 32783 using invented content; an access-shell paste is rejected. Native OS copying and live provider compatibility remain for the user's manual check. Current temporary review and reports are `/tmp/noms016-sally-review.json`, `/tmp/noms016-sally-semantic.{json,md}` and `/tmp/noms016-sally-corpus.{json,md}`; retain externally if needed. All work remains in the canonical NOMS-016 worktree; unrelated generated CSS is excluded.

The user postponed Sally's manual review and requested another provider. Simply Recipes was implemented next; neither provider is marked human-verified. Its exact HTTP(S) bare/www host opts into measured extensions of the shared Serious Eats/Dotdash article structure. Existing Serious Eats behavior is retained unchanged. Simply Recipes prefers an explicit card title when present and exposes a differing article title for confirmation, retains named and heading-only steps, keeps inline tips at their step position, and separates explicit recipe/card notes. Adjacent inline spans preserve word boundaries from the copied source. Mixed fractions, jar/block package units and singular pinch measures are supported; conversions and preparation suffixes stay literal. Only the exact observed review call-to-action is excluded, not arbitrary text mentioning reviews.

All 100 saved Simply Recipes captures activate the adapter (previously none). Accounting preserves all 985 ingredient rows across 114 groups, 442 steps, 11 inline callouts and 65 notes, excluding 30 exact review prompts. All 1,400 other-provider outcomes are identical to `69ac75c`; full integrity-verified structural acceptance is 1,000/1,500 with zero invalid candidates or unexpected errors. Twenty-two tests pass, including tests-first fixes for heading-only steps, misplaced notes, cooking text inside controls/ads, nested card metadata, heading punctuation and singular pinch. Independent review passes after the scoped metadata repairs. A separate three-candidate agent review finds 12/12 correct core fields; the original 27-candidate benchmark remains unchanged, not combined into a misleading historical denominator. Browser session rich paste → preview → private draft → title autosave → reload succeeds on port 32783 using invented content; an access shell is rejected. Manual live/native clipboard checks for Sally's and Simply Recipes remain pending. Changes stay in the canonical NOMS-016 worktree, with unrelated generated CSS excluded. See the real-corpus and semantic reports for exact commands and temporary external evidence.

### Sequential Food Network, Bon Appétit and Epicurious rollout — 2026-10-01

The user confirmed Sally's Baking Addiction and Simply Recipes working in the UI, then requested Food Network, Bon Appétit and Epicurious in that order. These three adapters are now implemented sequentially in the existing canonical `codex/NOMS-016-recipe-import-export` worktree; unrelated generated CSS remains excluded. This new checkpoint supersedes the earlier pending human-review status for Sally's and Simply Recipes, without retrospectively changing their implementation evidence.

| Provider | Saved captures accepted | Ingredient rows | Instruction steps | Explicit notes | Key handling |
| --- | ---: | ---: | ---: | ---: | --- |
| Food Network | 100/100 | 1,122 | 418 | 29 | Owned recipe card, responsive metadata agreement, extended article/card titles, ingredient/direction phases, qualified times and supplementary yields |
| Bon Appétit | 100/100 | 1,203 | 508 | 73 | Quantity/description sibling pairs, coauthors, numbered actions inside grouped list items, separate variations/make-ahead/editorial notes and nutrition metadata |
| Epicurious | 100/100 | 1,123 | 432 | 47 | Full-row ingredients, dotted-unit compound expressions, multi-paragraph numbered actions, explicitly numbered make-ahead actions and one measured empty video-teaser shell |

Exact HTTP(S) bare/www hosts and complete visible ownership are required; no URL fetching, HTML execution, image downloading or access-shell bypass is introduced. Bon Appétit's photo-only numbered step remains a literal step heading with low confidence and an explicit source-review warning. Output counts and ranged yields do not become numeric servings; qualified durations stay literal with a null numeric value. Active time, supplementary yields and nutrition remain preview-only unclassified metadata in the current mapper. Package expressions can remain conservatively in ingredient names rather than becoming guessed units.

Tests-first implementation and independent reviews pass after fixing dropped-content/ownership cases, conflicting metadata, inherited hostname dispatch and 21 Epicurious compound-quantity splits. The new provider suites contain 18, 25 and 17 tests respectively; all 210 cumulative provider tests pass. Final source accounting verifies all 300 requested captures and exact equality of the other 1,200 outputs against `4ed8a7d`. Integrity-verified full structural acceptance is 1,300/1,500, with zero invalid candidates or unexpected errors. BBC Good Food and King Arthur Baking remain outside this rollout. Structural acceptance and row retention are not provider-wide semantic-accuracy claims.

On the restarted canonical instance at port 32783, each invented provider-shaped rich clipboard paste previews, creates a private draft only after confirmation, autosaves a title edit and survives reload with its imported fields and consented commentary intact. Access-shell pastes refuse without a confirmation button. All test drafts remain unpublished. A development rebuild copied a truncated parser asset while edits were ongoing; restarting after source stabilization and checking the served module's syntax resolved this transient loading failure. Browser cache settings were restored after uncached verification. These are browser-session clipboard tests, not new native OS/live-provider fidelity checks.

`just fmt check lint`, the cumulative parser/provider/fixture/bridge/evaluator gates and eight focused Rust mapping tests pass. Full CI/database tests were not rerun. Commands, source-review evidence and remaining limitations are recorded in the real-corpus and semantic reports. These three providers are ready for the user's manual review on the current import page.

The separate nine-candidate independent agent source-to-output review has 28/36 correct core fields: three candidates usable without correction and six after correction, none unusable. Remaining cleanup includes package/alternative quantity splitting, a source-marked attribution retained as a method step, one missing image credit and the photo-only step. The historical benchmark is unchanged and not combined with this purposive subset. See the semantic report for field labels and digest-bound reproducibility.

### Live Bon Appétit/Epicurious layout correction — 2026-10-01

The user's Food Network manual check passed, but two live Condé pages rejected. Their introduction and recipe card are sibling regions inside an explicit article/content background, not one introduction container. `6115345` supports this measured ownership variant while retaining complete-marker and visibility/nav defenses. Ten new tests pass; all 1,500 saved HTML/source-URL outcomes remain deeply unchanged. Both live DOM captures now retain every owned ingredient/procedure paragraph, and the exact tested parser is deployed on the canonical port-32783 instance. Independent review, cumulative parser/bridge/fixture/evaluator gates and `just fmt check lint` pass. Raw captures stay external; unrelated CSS is excluded.

Fresh browser preview/native clipboard verification is still pending because browser automation hit focus timeouts/client blocking; no browser protection was bypassed. Refresh the import page, supply the recipe source URL, then re-copy/paste the page. The live DOM success is not a claim about the exact clipboard bytes the user originally pasted. No drafts were created or published. See the real-corpus report for commands, deployment hashes and limitations; full database CI was not rerun.

## First end-to-end clipboard integration

Provider expansion is paused while the existing parser is connected to the application. The first slice adds an authenticated Import Recipe page: supply an optional source URL, paste the full page, preview the extracted fields and warnings, then explicitly create a private draft and open the existing recipe editor. Editing, autosaving and publishing use the existing draft workflow; preview alone must not persist a recipe or raw clipboard data.

The client reads `text/html` and `text/plain` from the paste event. It loads the existing parser as a local application asset, with bounded input and parsing time. Changing the source or paste invalidates the previous preview. Clipboard content must remain text, never executable markup, and parsing must not fetch the supplied recipe URL or remote resources.

The candidate-to-editor mapping preserves amounts, units, ingredient order, instruction nesting, supported durations, servings and equipment. An explicit option carries source information, recipe notes, group labels and otherwise unmapped total-time/yield text into commentary. Ingredient groups become one ordered ingredient list; their labels remain in the optional commentary. Remote image references are disclosed in preview but are not downloaded or saved in this slice. The preview shows the exact commentary and normalized values to save, and explains that corrections happen in the editor before publishing.

Acceptance requires an actual captured recipe to travel through paste, preview, draft creation, editing, publishing and reload; confirmation of private visibility and no draft before confirmation; plain-text fallback and malformed-input recovery; stale-preview cancellation; and regression checks on existing recipe creation. Parser expansion and remote image import remain separate follow-ups.

Implementation checkpoint: `9ca0753` adds the import route, local worker bridge, preview and private-draft mapping. The worker accepts up to 4 MiB of combined UTF-8 clipboard content and has a 20-second deadline. Controls stay disabled until clipboard listeners are installed. Independent code review passes after fixing initialization-time HTML loss and incomplete saved-metadata preview. Nine focused Rust mapping/preview tests and three bridge tests pass, as do WASM/server compilation and the existing schema/parser regression targets. The broad CI gate still encounters 26 existing image-editor Clippy errors; the server suite also encounters two unchanged upload tests whose storage client is not initialized (526 passed, two failed, two ignored in that run). These are not import failures, but the full CI gate is not green.

Live end-to-end validation is still required before calling this slice complete. The initial fresh-instance attempt was blocked because Rancher Desktop was stopped and neither configured Docker socket existed. Headless execution of the actual BudgetBytes Homemade Meatballs clipboard pair through the bridge succeeds, but does not establish browser paste behavior, draft persistence, editing or publishing.

### Browser validation follow-up — 2026-09-12

After Rancher Desktop was started, a fresh `noms-noms016-browser-sep12` instance ran at `http://127.0.0.1:32768`. The in-app browser API was unavailable, so validation used the connected Chrome browser. The actual BudgetBytes HTML/plain-text capture was placed on the browser clipboard and pasted with Ctrl+V. Preview left the database at zero recipes/drafts/versions; confirmation created exactly one draft. Editing its title and publishing produced a private recipe and one version, removing the draft. Direct comparison with the existing parser output confirmed exact ingredient amounts/units/names/order, instruction text/nesting and equipment, with 25 prep minutes, 30 cook minutes, eight servings, source commentary and no images.

Empty-input and invalid-URL recovery, plain-text preview, disabling context, source-change invalidation and leaving/reopening the import route were exercised. A second real Tasty capture previewed ten ingredients and nine steps, with 20 prep minutes, 15 cook minutes and the original twelve-cookie yield retained (servings explicitly missing); it was not saved. The existing New Recipe route and its import link also worked. The nine Rust import checks and three bridge checks remain green.

The complete acceptance gate remains open: hard-reloading the published private recipe showed “Recipe not found”, while opening the same persisted recipe through the dashboard worked. The existing recipe-detail/authentication initialization path is unchanged by this integration; its resource captures the initial authentication snapshot before the client profile update, which is a likely cause to investigate. No authentication or recipe-detail fix was made as part of this browser test. Import persistence is verified, but direct reload must be repaired and retested before declaring the full end-to-end slice complete.

### NOMS-020 application rebase — 2026-09-30

Rebased all 55 NOMS-016 commits onto `codex/NOMS-020-checkpoint-8` at `52da873`, the combined NOMS-020 application checkpoint (also shared by the visual-design branch). The original tip `9d74b42` remains recoverable as `backup/NOMS-016-before-NOMS-020-20260930`. Shared-file conflicts retain NOMS-020's generated routing, redesigned dashboard, editor identity, authentication/recovery behavior, expanded planning document and security/schema CI checks. The import entry point now appears in the dashboard workspace sidebar; the new-recipe editor retains its NOMS-020 icon and gains the import action.

The parser, clipboard bridge, candidate mapping, import page and recipe fixtures are byte-for-byte unchanged from the original branch. `just ci` passes, including WASM/server checks, both Clippy targets, the server test suite, schema/security gates and the cumulative import/provider/bridge targets. The final upstream documentation-only checkpoint does not change the validated source or build inputs. The broader optional parser acceptance suite still has its pre-existing unsupported-access-shell failure (59 of 60 combined JavaScript tests pass); this case is outside the cumulative CI slice and was not changed during rebasing.

The existing import integration compiles against the current NOMS-020 draft/editor contracts. The next integration checkpoint is a browser rerun on this updated application, especially direct private-recipe reload, before declaring the earlier end-to-end acceptance gate complete.
