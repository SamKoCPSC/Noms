# NOMS-013: Marketing Pages & 3D Kitchen Experience — Implementation Plan

**Created:** 2026-07-16
**Depends on:** NOMS-003 application shell and design tokens; the current NOMS-008 recipe routes for feature links and demonstrations
**Coordination:** NOMS-012 may later replace the home-page Explore CTA with a search-first CTA. NOMS-013 must use canonical application routes and must not duplicate product functionality inside the marketing pages.
**Approach:** Replace the placeholder home page with a focused product landing page, add content-rich About and How-to routes, and progressively enhance those server-rendered pages with route-scoped Three.js scenes. Build the About page around an authored glTF kitchen and a deterministic scroll timeline, while retaining a complete, accessible, low-cost experience when WebGL, JavaScript, or motion is unavailable.

---

## Implementation contract

### Goals

- Explain what Noms is, who it is for, and why recipe history, organization, discovery, and collaboration are useful.
- Give first-time visitors an obvious path to sign in, explore recipes, or begin creating a recipe.
- Provide an instructional How-to page that demonstrates the current product workflow without pretending that planned features already exist.
- Make the About page a memorable guided kitchen tour whose camera, lighting, labels, and copy advance as the user scrolls.
- Reuse the existing warm kitchen palette, Fredoka/Nunito typography, light/dark themes, navigation, footer, and application routes.
- Replace generic emoji and text-glyph controls with a cohesive, original Noms icon family that also supports the new marketing pages.
- Keep the HTML content meaningful and navigable before the 3D runtime loads.

### Non-goals

- Rebuilding Dashboard, Explore, recipe CRUD, collections, search, authentication, or settings.
- Adding a CMS, analytics platform, user-generated 3D content, general-purpose scene editor, or backend API.
- Making WebGL a prerequisite for reading the pages or reaching a call to action.
- Modeling every kitchen prop at photorealistic fidelity. The target is a stylized, warm, optimized Noms kitchen.
- Advertising roadmap-only behavior as currently available. Future concepts may appear only in a clearly labelled “Where Noms is going” section.

### Public routes and page responsibilities

| Route | Component | Responsibility |
|---|---|---|
| `/` | `Home` | Product promise, high-level feature story, visual proof, and primary conversion paths |
| `/about` | `About` | Background, principles, audience, differentiators, and the scroll-driven 3D kitchen tour |
| `/how-to` | `HowTo` | Task-oriented instructions for creating, editing, organizing, discovering, and sharing recipes |

All three pages are public and remain under `AppLayout`. Add About and How-to links to the desktop navigation, mobile drawer, and footer. Preserve authenticated application links. On narrow navigation widths, prioritize `Home`, `Explore`, `About`, and `How to`; keep task links available from the existing drawer/user flows rather than allowing the desktop header to wrap.

### Content truth source

Page copy lives in typed Rust data beside the page/component that renders it, not embedded in the 3D scene or stored as textures. Each marketed feature has a title, concise description, availability state, destination route, and optional visual cue. Initial “available now” content must be reconciled against the routes and functionality present when the checkpoint starts.

The initial story should cover:

- Create and edit structured recipes with images, ingredients, instructions, timing, servings, equipment, and tags.
- Keep a personal recipe library and control recipe visibility.
- Organize recipes into collections, only once NOMS-011 is actually present; otherwise label it as upcoming or omit it.
- Discover public recipes and creator profiles.
- Preserve the long-term “living recipe” vision—history, drafts/branches, and community remixing—only where the corresponding implementation is available, or in the explicitly future-facing About section.

Do not hard-code user-specific claims or counts. Use “Start creating” for signed-out users and “Open dashboard” for authenticated users, derived from the existing auth context.

### Visual direction

- **Mood:** a sunlit, slightly whimsical working kitchen rather than a glossy appliance advertisement.
- **Shapes:** rounded cabinetry, tactile recipe cards, ceramic surfaces, soft food forms, and restrained neumorphic panels.
- **Palette:** existing oatmeal surfaces, terracotta accents, basil greens, and warm charcoal dark mode.
- **Motion:** slow, purposeful camera and object movement. Content transitions explain the product; they are not ornamental loops competing with reading.
- **2D/3D relationship:** headings, body copy, buttons, landmarks, and focus states remain semantic DOM. Canvas imagery supports them and never contains essential text.

### Custom icon system

Create a small, original SVG icon family rather than adopting an icon font or shipping platform-dependent emoji. The family should feel related to the 3D kitchen without becoming literal or difficult to scan.

#### Art direction

- **Character:** friendly, tactile, and slightly rounded, like marks drawn onto a recipe card with a confident felt-tip pen.
- **Geometry:** 24×24 view box, primarily 1.75–2px rounded strokes, rounded joins/caps, consistent optical padding, and simplified silhouettes that remain clear at 16px.
- **Kitchen motif:** use restrained culinary cues—recipe-card corners, utensil handles, bowl arcs, pantry labels, steam curves—only when they reinforce meaning. Universal actions such as close, back, forward, warning, and search must remain immediately conventional.
- **Color:** interface icons inherit `currentColor`. Selected, status, empty-state, and marketing variants may use a documented two-tone accent layer through CSS variables. Do not encode light/dark colors directly into SVG paths.
- **Depth:** marketing/empty-state variants may add one offset contour or soft filled plane that echoes the 3D visual language. Compact controls remain flat to preserve clarity.
- **Motion:** a limited set may animate through CSS transforms or stroke changes for state transitions. Motion is optional, never essential, and disabled by `prefers-reduced-motion`.

#### Initial icon family

| Group | Icons/design direction |
|---|---|
| Brand/navigation | `noms-mark` (crossed tasting spoon and recipe-card corner), `menu` (stacked recipe lines), `home`, `dashboard` (four recipe tiles), `explore` (compass plate), `settings` |
| Recipe actions | `recipe`, `add-recipe` (card with plus), `edit`, `duplicate/fork` (branching card), `history` (clock around card), `scale` (measuring spoon/arrows), `favorite` (soft star), `share` |
| Organization/discovery | `collection` (tabbed recipe box rather than a generic OS folder), `search`, `filter` (funnel/strainer reference), `sort`, `tag`, `profile` |
| Images and editing | `camera`, `image`, `upload`, `crop`, `rotate-left`, `rotate-right`, `move-left`, `move-right`, `remove` |
| System/status | `close`, `back`, `forward`, `check`, `warning`, `error`, `info`, `external-link`, `sun`, `moon`, `link` |
| Marketing/About | `capture`, `organize`, `evolve`, `discover`, `community`, and six kitchen-landmark spot illustrations aligned with the About tour beats |

The brand mark is an icon-system deliverable, not permission to replace the Noms wordmark without a separate review. Keep the current wordmark text alongside the new mark until its legibility is approved at navbar and favicon sizes.

#### Technical contract

- Store canonical, optimized source SVGs under `assets/icons/` and document the grid, stroke, naming, licensing, and export process in `assets/icons/README.md`.
- Render interface icons through a typed Rust `Icon` component and `IconName` enum. Prefer an internal SVG symbol sprite or generated match-based paths so repeated icons do not duplicate large inline markup; choose the strategy after verifying SSR, asset fingerprinting, and client navigation.
- `Icon` accepts semantic size classes and optional decorative/accessible behavior. It inherits surrounding text color and sets `aria-hidden="true"` when a nearby visible label supplies the name.
- Icon-only buttons retain explicit accessible names through `aria-label` or visually hidden text. Tooltips may supplement those names but never replace them.
- Do not use Unicode arrows, multiplication signs, emoji, CSS background images, or text characters as control icons after an equivalent custom icon ships.
- Keep provider trademarks such as Google/GitHub sign-in marks separate from the Noms family and follow their brand requirements; do not redraw them in the custom style.
- Illustrative two-tone icons and tour landmarks may be separate SVG assets, but must share the same grid/radius/stroke rules and have raster fallbacks only where required by social metadata.

Proposed additions:

```text
assets/
  icons/
    README.md
    noms-icons.svg
    source/
      *.svg
src/
  components/
    base/
      icon.rs
```

### 3D architecture

Use Three.js as a client-only JavaScript module managed through npm, with one small Rust/Dioxus interop boundary. Do not bind the full Three.js API into Rust or include it in the server bundle.

Proposed structure:

```text
assets/
  3d/
    kitchen.glb
    kitchen-poster.webp
    home-hero.glb
    home-hero-poster.webp
  js/
    noms-3d.js
src/
  components/
    marketing/
      mod.rs
      feature_section.rs
      marketing_cta.rs
      scene_canvas.rs
      step_guide.rs
  pages/
    home.rs
    about.rs
    how_to.rs
  utils/
    marketing_3d.rs
```

`SceneCanvas` renders a labelled canvas host, poster fallback, loading status, and recovery message. `marketing_3d.rs` owns mount/update/dispose calls under `cfg(target_arch = "wasm32")`. `noms-3d.js` owns Three.js imports, render loops, glTF loading, resize observation, visibility pausing, WebGL capability detection, and GPU resource disposal.

Each mounted scene returns an opaque instance ID. Route unmount must cancel animation frames, disconnect observers/listeners, dispose geometries/materials/textures/renderers, and remove its registry entry. Client navigation back to a page must create one fresh scene, never a second animation loop on the old canvas.

Bundle Three.js and its selected addons locally during the normal asset build. Do not fetch the runtime or models from a CDN. Split the marketing 3D module so application routes do not pay its parse/transfer cost, and lazy-load each scene when its canvas approaches the viewport.

### About scroll-tour contract

The kitchen is one optimized glTF/GLB scene with intentionally named landmarks and authored camera targets. Initial tour beats:

| Beat | Kitchen landmark | Story |
|---|---|---|
| 1. Welcome | Kitchen doorway / wide establishing view | Noms began with the idea that recipes should live and evolve |
| 2. Capture | Prep counter and open recipe card | Turn cooking knowledge into structured, reusable recipes |
| 3. Organize | Pantry shelves and labelled containers | Keep a personal library understandable as it grows |
| 4. Evolve | Cutting board with layered cards/variations | Preserve history and experiment without losing the trusted version |
| 5. Share | Serving island with multiple place settings | Discover, attribute, and build on community knowledge |
| 6. Begin | Warm table / final wide shot | CTA to explore or create |

The About page uses a sticky scene column/layer and six semantic content sections. An `IntersectionObserver` identifies the active section and sends a normalized beat/progress value to the scene. The renderer interpolates between authored camera position, target, focal length, lighting, and optional object-emphasis values. Do not drive a Dioxus rerender for every scroll pixel.

Scrolling must always control page progress; do not trap wheel/touch input, hide the browser scrollbar, or replace native scrolling. Labels associated with landmarks are DOM elements linked to section headings with `aria-describedby`; any projected 3D callout is supplementary and hidden from assistive technology.

### Graceful-degradation modes

1. **Full:** WebGL available, normal motion preference, adequate viewport; render the complete scene and camera timeline.
2. **Reduced motion:** Render a static camera per beat or the poster; switch without animated interpolation and disable idle motion/parallax.
3. **Constrained/mobile:** Use reduced pixel ratio, simplified lighting/material settings, and fewer decorative meshes, or fall back to per-beat poster crops if the performance budget cannot be held.
4. **No WebGL/load failure/JavaScript disabled:** Show the responsive poster and all DOM content/CTAs. Announce a concise non-blocking load failure in the scene status region.

The application must respect `prefers-reduced-motion`, pause when `document.visibilityState` is hidden, stop rendering when the canvas is well outside the viewport, and avoid automatic audio.

### Performance budgets

Measure against a production build on a representative mid-range mobile device and desktop, with cache disabled and throttling documented.

- Home page initial critical JS excludes Three.js; the 3D chunk begins loading near the hero/feature scene only after HTML is interactive.
- Compressed `home-hero.glb` target: <= 750 KB; compressed `kitchen.glb` target: <= 3 MB; each poster target: <= 250 KB.
- Cap device pixel ratio at `1.5` desktop and `1.25` mobile; target 60 fps desktop and a stable 30 fps minimum mobile while the scene is active.
- No avoidable layout shift from scene loading: canvas/poster containers have explicit aspect ratio or reserved viewport height.
- Use KTX2/Basis textures and Meshopt or Draco geometry compression only after verifying browser decoding and build/deployment paths. Prefer Meshopt when comparable because of its decoding profile.
- Avoid dynamic shadows on mobile. Bake ambient occlusion/light where practical, share materials, instance repeated props, and keep draw calls documented in the asset manifest.
- Record final GLB size, texture memory estimate, triangle count, material count, and draw calls in `assets/3d/README.md`.

### Accessibility and SEO

- One `h1` per page; sequential headings, semantic sections, descriptive link text, visible keyboard focus, 44px touch targets, and WCAG AA text contrast.
- Decorative canvases use an appropriate accessible name and fallback description; essential instructions never require interpreting a spatial scene.
- Add per-route title, description, canonical URL, and Open Graph metadata using Dioxus document-head components. The server-rendered response must contain the page’s primary copy and metadata.
- Keyboard users can reach every CTA and How-to step without interacting with the canvas. If any scene control is introduced, it must be operable by keyboard and have a visible text equivalent.
- Test at 200% zoom, with reduced motion, in forced-colors/high-contrast mode, and with a screen-reader-oriented DOM inspection.

---

## Checkpoint 1: Content inventory, wireframes, and asset brief

**Files:** `roadmap/implementation-plans/NOMS-013-marketing-pages-3d-experience.md`, new `assets/3d/README.md`, optional design references under `docs/design/NOMS-013/`

### Tasks

- Audit the feature availability at implementation time and create the typed content inventory for Home, About, and How-to.
- Produce desktop and mobile wireframes showing page sections, CTA hierarchy, sticky About behavior, fallback posters, and the transition into the existing footer.
- Define the six About beats with final copy, camera framing thumbnails, active DOM section, landmark node names, and target camera transforms.
- Write a kitchen asset brief covering dimensions, topology, texture sizes, light-baking assumptions, dark-theme treatment, compression, and required named nodes.
- Decide whether the kitchen is authored in Blender or sourced from appropriately licensed assets. Record creator, license, source URL, modifications, and attribution requirements for every non-original asset.
- Establish baseline Lighthouse/WebPageTest-style measurements for the current home page and record the chosen test devices, viewport sizes, network profile, and browser.

### Verify

- Every factual feature claim maps to a working route or is explicitly labelled future-facing.
- Every tour beat works as a readable linear page outline without the kitchen scene.
- Mobile wireframes do not depend on a sticky canvas occupying most of the viewport.
- Asset licensing and redistribution rights are documented before files enter the repository.

**Risk:** Medium. Content/scene mismatch is expensive to repair after camera paths and models are authored.

---

## Checkpoint 2: Public routes, marketing shell, and metadata

**Files:** `src/main.rs`, `src/pages/mod.rs`, `src/pages/home.rs`, new `src/pages/about.rs`, new `src/pages/how_to.rs`, `src/components/navbar.rs`, `src/components/footer.rs`, `src/components/app_layout.rs`, `assets/main.css`

### Tasks

- Add typed `/about` and `/how-to` routes before the catch-all route and export both page components.
- Add desktop, drawer, and footer navigation links with current-route indication and accessible labels.
- Add a marketing-page modifier to `AppLayout` or page-level full-bleed wrappers so the immersive About canvas can reach the viewport edges without changing application-page content widths.
- Implement complete SSR-first page skeletons with semantic headings, text, CTAs, poster placeholders, and route-specific metadata.
- Keep Navbar and Footer usable over each page’s light and dark backgrounds; use design tokens rather than page-specific hard-coded theme colors.
- Add CSS foundations for marketing section spacing, full-bleed regions, sticky layouts, responsive type, and reserved scene aspect ratios.

### Verify

- Direct requests and client navigation to `/`, `/about`, and `/how-to` render correctly in server and web builds.
- View-source/SSR output contains page titles, descriptions, primary headings, explanatory copy, and working links.
- Navigation does not wrap or overflow at supported breakpoints; the mobile drawer closes after either new route is selected.
- Unknown routes still reach `NotFound`.
- `cargo fmt --check`, `cargo check`, and `git diff --check` pass.

**Risk:** Low. This establishes useful pages before any 3D dependency is introduced.

---

## Checkpoint 3: Shared marketing components and Home page

**Files:** `src/pages/home.rs`, new `src/components/marketing/*`, `src/components/mod.rs`, `assets/main.css`

### Home composition

1. Hero: concise promise, supporting sentence, auth-aware primary CTA, Explore secondary CTA, and a reserved 3D/product visual.
2. Value strip: create, organize, evolve, and discover.
3. Alternating feature stories: real UI excerpts or illustrative recipe cards paired with concise copy.
4. “Built for the way recipes change” differentiator section.
5. Community/discovery section grounded in currently available public features.
6. Final CTA with a quiet path to the How-to page.

### Tasks

- Introduce reusable feature-section, CTA, status badge, scene-host, and instructional-step primitives with typed props.
- Make CTA destinations auth-aware without showing user-specific content in the server-rendered marketing copy.
- Replace the current inline-styled Home placeholder with section classes and shared design tokens.
- Use CSS transforms/perspective for small card-depth effects where WebGL adds no meaningful value.
- Add robust responsive states: stacked copy/visuals, logical source order, no horizontal overflow, and no content obscured by the fixed navbar.

### Verify

- Signed-out and authenticated CTA destinations and labels are correct.
- The page remains coherent with images disabled and contains no essential copy in generated imagery.
- Light/dark screenshots at mobile, tablet, and desktop match the approved wireframes.
- Automated route/component tests cover CTA selection and feature availability labels where practical.

**Risk:** Medium. The landing page can become visually noisy unless CTA hierarchy and section pacing remain disciplined.

---

## Checkpoint 4: Route-scoped 3D runtime and scene host

**Files:** `package.json`, lockfile, asset build configuration, new `assets/js/noms-3d.js`, new `src/utils/marketing_3d.rs`, `src/utils/mod.rs`, `src/components/marketing/scene_canvas.rs`, `Cargo.toml` only for additional browser API features actually required

### Tasks

- Add a pinned Three.js dependency and a reproducible script that bundles the route-scoped module into a fingerprintable local asset.
- Implement the narrow `mount_scene`, `set_scene_progress`, `set_scene_theme`, and `dispose_scene` interface.
- Build `SceneCanvas` lifecycle handling for SSR, hydration, near-viewport lazy loading, loading state, poster fallback, error recovery, resize, theme changes, and unmount.
- Detect WebGL support before importing heavy scene code. Treat model failure as a visual downgrade, not a page failure.
- Add renderer safeguards: DPR caps, transparent/solid background contract, color-space/tone-mapping configuration, visibility pause, and context-loss handling.
- Add development-only diagnostics for renderer count, active animation loops, frame time, draw calls, triangles, and texture count.

### Verify

- Application routes that never mount `SceneCanvas` do not request the Three.js chunk or GLB assets.
- Navigating repeatedly among Home/About/How-to leaves at most one renderer and animation loop per visible scene.
- Resizing, theme switching, tab hiding, WebGL context loss, and asset-load failure do not panic the Dioxus app.
- A production build references local fingerprinted assets and requires no 3D CDN at runtime.
- Baseline application bundle and route performance regressions are recorded.

**Risk:** High. Lifecycle leaks and accidental eager loading would degrade every app session.

---

## Checkpoint 5: Home 3D hero enhancement

**Files:** `assets/3d/home-hero.glb`, `assets/3d/home-hero-poster.webp`, `assets/3d/README.md`, `assets/js/noms-3d.js`, `src/pages/home.rs`, `assets/main.css`

### Tasks

- Create an optimized hero scene: a tactile recipe card/book with a few recognizable kitchen props and restrained depth/lighting.
- Add slow idle motion plus subtle pointer parallax on fine-pointer devices. Clamp motion and never require dragging to understand the visual.
- Synchronize the scene palette/light treatment with theme changes without reloading the model.
- Render the poster first and crossfade only after the first stable WebGL frame so the hero never flashes blank.
- Disable idle/parallax motion under reduced-motion and avoid continuous rendering when the scene is static.

### Verify

- Hero GLB, poster, and texture budgets are met and documented.
- CTA interaction is never blocked by the canvas; pointer behavior does not cause page scroll or hover traps.
- Low-power/mobile mode holds the agreed frame-time target or selects the poster fallback.
- The hero has no layout shift and remains visually complete before the model loads.

**Risk:** Medium. The hero must improve comprehension and brand feel without delaying the first interaction.

---

## Checkpoint 6: About page scroll-driven kitchen tour

**Files:** `assets/3d/kitchen.glb`, `assets/3d/kitchen-poster.webp`, optional responsive poster crops, `assets/3d/README.md`, `assets/js/noms-3d.js`, `src/pages/about.rs`, `assets/main.css`

### Tasks

- Author/import, optimize, and validate the complete kitchen scene against the asset brief and named-landmark contract.
- Implement six semantic story sections and a sticky canvas layout for wide screens, with an intentionally different stacked/mobile presentation.
- Use `IntersectionObserver` plus bounded section progress to select tour beats; interpolate camera and emphasis state in the renderer without per-frame Rust/Dioxus state updates.
- Prevent abrupt jumps on refresh, deep anchor navigation, browser back/forward, and viewport rotation by deriving the initial beat from current document geometry.
- Highlight the active landmark through lighting/material emphasis while keeping decorative motion secondary.
- Add reduced-motion, mobile/constrained, no-WebGL, and load-error modes defined in the implementation contract.
- Add optional anchor links for each story beat so content is linkable and keyboard/screen-reader navigation does not depend on scrolling through a long sticky region.

### Verify

- Native wheel, keyboard, touch, scrollbar dragging, anchor links, and browser restoration all reach the same six sections.
- Camera positions never clip through geometry at supported aspect ratios and text remains legible over every frame.
- Reduced-motion mode has no camera tweening, parallax, or idle animation.
- The complete story and final CTA remain available when the canvas is removed from the DOM.
- Kitchen GLB and runtime scene metrics meet the documented budgets on desktop and mobile test devices.

**Risk:** High. Scroll choreography, responsive camera framing, asset weight, and GPU variability require early device testing.

---

## Checkpoint 7: How-to instructional page

**Files:** `src/pages/how_to.rs`, `src/components/marketing/step_guide.rs`, supporting optimized images/illustrations under `assets/how-to/`, `assets/main.css`

### Instruction structure

- Getting started and signing in.
- Creating a recipe: metadata, ingredients, instructions, images, and visibility.
- Editing/scaling/versioning only to the extent currently shipped.
- Organizing into collections only if NOMS-011 is shipped.
- Finding recipes through Dashboard/Explore and search only if NOMS-012 is shipped.
- Sharing public/unlisted recipes and visiting public profiles.

### Tasks

- Build a table of contents with anchor links and a clear “available now” task sequence.
- Give each task prerequisites, numbered steps, expected result, and direct link into the relevant product route.
- Use lightweight depth treatments or a small reusable 3D object only where it clarifies a control or concept; default to optimized screenshots/illustrations for procedural accuracy.
- Add callouts for visibility/privacy implications and irreversible actions where applicable.
- Ensure examples and screenshots use fictional, non-sensitive data and are refreshed when the referenced UI changes.
- Add a final troubleshooting section for sign-in, image upload, recipe visibility, and finding saved work, limited to verified behavior.

### Verify

- A new user can complete each documented task against the current app without undocumented steps.
- Anchors, table of contents, and route links work with keyboard and client navigation.
- Screenshots match the current UI and include useful alt text rather than duplicating nearby prose.
- Instructions do not claim NOMS-010/011/012 behavior before those issues are implemented.

**Risk:** Medium. Instructional content becomes harmful if it drifts ahead of the product.

---

## Checkpoint 8: Custom icon family and application migration

**Files:** new `assets/icons/README.md`, new `assets/icons/source/*.svg`, generated/optimized `assets/icons/noms-icons.svg`, new `src/components/base/icon.rs`, `src/components/base/mod.rs`, `assets/main.css`, and current emoji/text-icon call sites in `src/components/` and `src/pages/`

### Tasks

- Inventory every visible emoji, Unicode glyph, text-only control, existing provider mark, favicon, and CSS-drawn icon. Classify each as brand, action, navigation, status, decorative, or third-party.
- Create a contact sheet showing the initial family at 16, 20, 24, 32, and 48px in both themes. Review silhouette recognition, stroke consistency, optical centering, and paired states before implementing the full set.
- Design the core set first: Noms mark, menu, close, back, forward, sun, moon, search, collection, recipe, camera, favorite, link, warning, check, and remove. Add feature/marketing icons only after these establish the visual grammar.
- Build the typed `Icon`/`IconName` API with standard sizes, `currentColor`, two-tone variables, decorative defaults, and an explicit accessible mode.
- Add a deterministic optimization/build step that strips editor metadata, preserves the view box, avoids generated IDs colliding in the DOM, and fails when required icons are missing or duplicated.
- Replace the current navbar/home `🍴`, menu `☰`, close `✕`, theme emoji, camera, search, collection, warning, empty-state, favorite, upload-status, and navigation-arrow glyphs with the new component.
- Replace icon-plus-label strings such as `📷 Click or drag images here` and `← Back to home` with separately rendered icon and text nodes so assistive technology reads only the intended label.
- Keep OAuth/provider artwork unchanged and visually balance it with the new family without recoloring or reshaping protected marks.
- Derive favicon/app-icon candidates from the approved Noms mark and verify them independently at 16px and against browser light/dark chrome.

### Design verification

- Each action icon is understandable without color and is distinct from neighboring actions; close/remove, back/undo, collection/recipe, and favorite/rating receive explicit confusion testing.
- Icons remain crisp at common CSS sizes without clipped strokes, uneven baselines, or theme-specific contrast failures.
- Selected, disabled, hover, focus, success, warning, and destructive states use the existing semantic tokens consistently.
- Marketing icons feel related to the 3D scene but do not introduce a second illustration style.

### Technical verification

- SSR output references valid SVG symbols/paths and icons remain visible after hydration and client navigation.
- Automated tests cover enum-to-symbol completeness and accessible/decorative rendering behavior.
- An emoji/Unicode-control audit leaves no accidental placeholder icons in migrated scopes; approved textual punctuation and third-party trademarks are excluded.
- Icon-only controls have accessible names, visible focus treatment, and 44px touch targets even when the glyph itself is smaller.
- The complete optimized core icon payload stays within a documented budget, with no icon font, external runtime, or network dependency.

**Risk:** Medium. A broad one-pass replacement can introduce ambiguous controls, so the core contact sheet and recognition review gate migration.

---

## Checkpoint 9: Cross-browser, accessibility, performance, and release gate

**Files:** page/component tests, `docs/manual-test-guide.md`, `assets/3d/README.md`, NOMS-013 visual baselines if the repository adopts screenshot testing

### Automated validation

- Add route rendering tests for Home, About, and How-to plus CTA/auth variants.
- Add browser tests for scene lazy load, fallback activation, scroll-beat selection, reduced-motion behavior, theme change, and disposal after navigation.
- Add browser/a11y coverage for custom icon rendering, icon-only accessible names, theme contrast, and sprite/path availability after client navigation.
- Run formatting, clippy/check, existing Rust tests, Tailwind/asset production builds, and `git diff --check`.
- Run an accessibility audit (for example axe) on all three routes in both themes; manually inspect landmarks and reading order.

### Manual matrix

- Current Chrome, Firefox, and Safari on desktop.
- Current Chrome on Android and Safari on iOS, including portrait/landscape changes.
- Light/dark theme, signed-in/signed-out, reduced motion, 200% zoom, keyboard only, slow network, offline-after-load, WebGL disabled, and simulated context loss.
- Navigation loops among public and application routes to detect leaked canvases, listeners, memory, or stale theme state.

### Release thresholds

- No critical accessibility issues and no keyboard traps.
- All content and CTAs work in no-WebGL mode.
- Production performance stays within the agreed page/asset/frame budgets; deviations require a recorded decision and fallback adjustment.
- No console errors during normal navigation, scroll, resize, theme change, or scene teardown.
- No generic emoji or text-glyph placeholders remain in the NOMS-013 migration scope, except documented third-party or textual content exclusions.
- Final copy and feature-status labels receive a product-truth review.

### Rollout

- Keep a runtime kill switch that forces poster mode for the Home and About scenes independently without removing page content.
- Release Home and How-to first if the kitchen tour misses its performance or accessibility gate; `/about` may ship with the poster-based story and enable the full tour later.
- Treat asset optimization, poster fallback, and teardown correctness as release requirements, not post-launch polish.

**Risk:** Medium. The fallback-first structure makes a partial rollout safe while the immersive layer is tuned.

---

## Definition of done

- `/`, `/about`, and `/how-to` are polished, responsive, server-rendered public pages with accurate content and working navigation.
- Home communicates Noms’ high-level value and offers clear, auth-aware next actions.
- About presents the six-part kitchen story, with a smooth scroll-driven 3D tour on capable devices and an equivalent semantic/poster experience everywhere else.
- How-to documents only shipped workflows with task-oriented steps and direct product links.
- 3D code and assets are route-scoped, locally hosted, optimized, observable, disposable, and guarded by independent poster-mode switches.
- The approved custom SVG family replaces placeholder control icons across the planned application scope through a typed, themeable, accessible component.
- Accessibility, reduced-motion, no-WebGL, mobile, SEO, theme, performance, and cross-browser criteria pass the release gate.
- 3D and icon asset sources/licenses, icon construction rules, and final scene metrics are documented, and the manual test guide includes NOMS-013 scenarios.

---

## Recommended implementation order

Checkpoint 1 → Checkpoint 2 → Checkpoint 3 → Checkpoint 4 → Checkpoint 5 → Checkpoint 6 → Checkpoint 7 → Checkpoint 8 → Checkpoint 9.

Icon concept work from Checkpoint 8 may begin alongside Checkpoint 1, but application migration should follow approval of the core contact sheet. Checkpoints 5 and 7 may proceed in parallel after the shared components and 3D runtime are stable. Keep Checkpoint 6 behind the kitchen asset brief and runtime disposal/performance proof; it is the largest technical and production risk in NOMS-013.
