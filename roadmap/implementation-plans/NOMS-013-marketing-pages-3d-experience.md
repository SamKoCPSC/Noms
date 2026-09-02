# NOMS-013: Marketing Pages & 3D Feasibility Experiment — Implementation Plan

**Created:** 2026-07-16
**Revised:** 2026-08-14
**Status:** Ready for implementation
**Depends on:** The current application shell, design tokens, authentication context, and canonical recipe/application routes
**Coordination:** NOMS-012 may later replace the Home Explore CTA with a search-first CTA. NOMS-013 must use canonical application routes and must not duplicate product functionality inside marketing pages.
**Approach:** Ship polished, conventional, server-rendered Home, About, and How-to pages using semantic HTML, responsive CSS, optimized screenshots, and original SVG illustrations. Do not put a 3D runtime or scene on production pages. After the complete traditional experience passes its release gate, conduct an isolated 3D feasibility experiment and record whether a separately scoped follow-up is justified.

---

## Outcome

NOMS-013 delivers a complete public marketing and onboarding experience without requiring WebGL, WebGPU, JavaScript animation, or a game/rendering engine.

The issue is complete when:

- `/`, `/about`, and `/how-to` are accurate, responsive, accessible, server-rendered public pages.
- Navigation, metadata, visual hierarchy, calls to action, and instructional content pass the release gate.
- The custom Noms icon family replaces placeholder glyphs in the public-page and navigation scope.
- No production page imports Three.js, `three-d`, Bevy, `wgpu`, or experimental 3D assets.
- A final isolated experiment compares viable 3D approaches and produces a written decision. Shipping 3D is not required to complete NOMS-013.

## Implementation baseline (verified 2026-08-14)

- Dioxus `0.7.x` fullstack SSR and hydration are configured. Routes live in `src/main.rs`; page exports live in `src/pages/mod.rs`.
- `/`, `/login`, `/dashboard`, recipe CRUD, version history, drafts, `/collections`, `/explore`, public profiles, and settings routes exist.
- Collections and recipe versioning/drafts may be described as available after their workflows are replayed in Checkpoint 1.
- Search remains conditional until a canonical `/search` route exists. NOMS-013 must not create a separate search implementation.
- Home is currently a small inline-styled placeholder. `Navbar`, its desktop/mobile/drawer variants, `Footer`, and `AppLayout` are the public integration points.
- Authentication context is available during SSR and hydrated on the client. CTA selection must be implemented as a pure, testable function.
- `assets/main.css` contains existing design tokens and component styles. `assets/tailwind.css` is generated and must not be edited manually.
- The repository has no browser-test runner, screenshot harness, JavaScript bundler, or 3D asset pipeline.

If this baseline changes before implementation begins, update this section and affected checkpoints before changing production code.

## Scope

### Goals

- Explain what Noms is, who it serves, and why structured recipes, history, organization, discovery, and collaboration are useful.
- Give visitors clear paths to sign in, explore recipes, create a recipe, or open their dashboard.
- Provide an About page that communicates the product story through an authored editorial layout rather than an interactive scene.
- Provide a task-oriented How-to page that documents only currently shipped workflows.
- Establish a recognizable visual language through typography, layout, CSS depth, screenshots, recipe-card compositions, and original SVG illustrations.
- Preserve meaningful HTML, metadata, navigation, and calls to action in the server-rendered response.
- End with a bounded 3D feasibility experiment that cannot delay or destabilize the production release.

### Non-goals

- Adding production WebGL/WebGPU canvases, 3D models, render loops, or renderer dependencies.
- Rebuilding Dashboard, Explore, recipe CRUD, collections, search, authentication, or settings.
- Adding a CMS, analytics platform, backend API, or database-backed marketing content.
- Advertising roadmap-only behavior as available.
- Replacing unrelated icons throughout every authenticated editor.
- Treating a successful 3D prototype as authorization to add it to production pages.

### Public routes

| Route | Component | Responsibility |
|---|---|---|
| `/` | `Home` | Product promise, feature story, visual proof, and primary conversion paths |
| `/about` | `About` | Background, principles, audience, differentiators, and future direction |
| `/how-to` | `HowTo` | Task-oriented instructions for using currently available product workflows |

All three routes remain under `AppLayout`. Add About and How-to to desktop navigation, mobile navigation/drawer, and Footer. Preserve authenticated application links and prevent desktop navigation from wrapping at supported widths.

## Fixed implementation decisions

- Page copy lives in typed Rust data beside the relevant page or shared marketing component.
- Marketing pages use ordinary DOM content and CSS. JavaScript is not required for comprehension, navigation, or visual completeness.
- Production visuals use checked-in SVG illustrations, CSS compositions, or WebP/AVIF screenshots with explicit dimensions.
- CSS transitions are limited to short interface feedback and simple reveal/hover states. They respect `prefers-reduced-motion` and never control reading progress.
- No scroll interception, parallax framework, animation timeline library, carousel dependency, or continuously running animation loop is introduced.
- Screenshots contain fictional, non-sensitive data and are accompanied by text that remains meaningful when images are unavailable.
- Canonical URLs derive from configured `BASE_URL`; production origin strings are not hard-coded.
- NOMS-013 owns icons used by Home, About, How-to, Navbar variants, Footer, and shared marketing components. Other glyph migrations are inventoried as follow-up work.
- Experimental code lives under `experiments/NOMS-013-3d/`, is excluded from production imports and build requirements, and may be removed without altering production pages.

## Content truth contract

Create `docs/design/NOMS-013/content-inventory.md` with one row per marketed claim:

| Field | Requirement |
|---|---|
| Page/section | Exact destination in Home, About, or How-to |
| Final copy | User-visible claim |
| Availability | `available` or `future` |
| Evidence | Working route and replayed workflow |
| CTA | Canonical destination, if any |
| Review result | Reviewer and date |

The initial story may cover:

- Creating and editing structured recipes with images, ingredients, instructions, timing, servings, equipment, tags, and visibility.
- Maintaining a personal recipe library.
- Organizing recipes into collections.
- Discovering public recipes and creator profiles.
- Version history, drafts, branching, remixing, or search only to the extent actually present when copy is approved.
- Long-term concepts only inside a clearly labelled future-facing section.

Do not hard-code user-specific claims or counts. Signed-out users see “Start creating”; authenticated users see “Open dashboard,” selected from the existing auth context.

## Visual and interaction direction

### Character

- Warm, editorial, tactile, and slightly whimsical rather than a generic SaaS landing page.
- Existing oatmeal surfaces, terracotta accents, basil greens, warm charcoal dark mode, Fredoka headings, and Nunito body text.
- Rounded recipe cards, layered paper edges, pantry labels, ceramic shapes, ingredient photography, and restrained neumorphic panels.
- Depth comes from CSS borders, shadows, gradients, transforms, masks, and overlapping SVG/HTML layers—not from a canvas.

### Home composition

1. Hero with concise promise, supporting copy, auth-aware primary CTA, Explore secondary CTA, and a static recipe/kitchen composition.
2. Value strip covering capture, organize, evolve, and discover.
3. Alternating feature stories using real UI excerpts or authored recipe-card illustrations.
4. “Built for the way recipes change” differentiator section.
5. Community/discovery section grounded in current public functionality.
6. Final CTA with a quiet path to How-to.

### About composition

Replace the former scroll-driven kitchen with a conventional editorial narrative:

| Section | Visual treatment | Story |
|---|---|---|
| Welcome | Wide static kitchen/recipe illustration | Recipes should live and evolve |
| Capture | Recipe card and prep-counter composition | Turn cooking knowledge into structured recipes |
| Organize | Pantry-label and collection-card composition | Keep a growing library understandable |
| Evolve | Layered version-card timeline | Preserve history and experiment safely |
| Share | Serving-table/community profile composition | Discover, attribute, and build on shared knowledge |
| Begin | Warm final CTA panel | Explore or start creating |

Each section uses semantic headings, concise copy, an optional optimized visual, and ordinary document flow. On wide screens, sections may alternate columns; on narrow screens, text precedes supporting visuals in logical source order.

### How-to composition

- Sticky table of contents only where viewport height permits; otherwise use an ordinary anchor list.
- Each task includes prerequisites, numbered steps, expected result, and a direct canonical route.
- Prefer accurate screenshots for procedures and SVG illustrations for concepts.
- Add verified privacy/visibility notes and troubleshooting only where current behavior supports them.

## Accessibility, SEO, and performance contract

- One `h1` per page; sequential headings, semantic landmarks, descriptive links, visible focus, 44px touch targets, and WCAG AA text contrast.
- Illustrations are decorative when nearby prose conveys the same meaning; informative screenshots receive concise useful alt text.
- Primary content and all CTAs are present in SSR output.
- Add per-route title, description, canonical URL, and Open Graph metadata using Dioxus document-head components.
- Pages work at 200% zoom, with reduced motion, forced colors/high contrast, keyboard-only navigation, images disabled, and CSS disabled enough to preserve logical reading order.
- Reserve image dimensions/aspect ratios to prevent avoidable layout shift.
- Hero/LCP imagery must be responsive and must not be lazy-loaded; below-fold imagery is lazy-loaded.
- No new production JavaScript dependency is required for the marketing experience.
- Establish and record baseline/result measurements with the same production `dx bundle --platform web --release` profile, device, viewport, throttling, cache state, and three-run median.

## Delivery sequence

| PR | Checkpoint | Merge condition |
|---|---|---|
| 1 | Content, wireframes, and visual inventory | Claims, layouts, asset sources, and baselines approved |
| 2 | Routes, navigation, shell, and metadata | SSR routes work without client JavaScript |
| 3 | Shared marketing UI and Home | Responsive Home and CTA tests pass |
| 4 | Traditional About page | Six-section editorial story passes responsive/a11y review |
| 5 | How-to page | Every procedure is replayed against the current app |
| 6 | Core icon system | Typed icon tests and scoped glyph audit pass |
| 7 | Production release gate | Automated/manual matrix passes |
| 8 | Isolated 3D feasibility experiment | Benchmark report and follow-up decision recorded |

Production release does not wait for a favorable result from Checkpoint 8. Checkpoint 8 begins only after Checkpoint 7 passes.

---

## Checkpoint 1: Content, wireframes, and asset inventory

**Files:** this plan, new `docs/design/NOMS-013/content-inventory.md`, wireframes under `docs/design/NOMS-013/`, and an asset manifest under `docs/design/NOMS-013/assets.md`

### Tasks

- Replay every marketed workflow against the current application and complete the content truth table.
- Write final Home, About, and How-to copy before producing illustrations or screenshots.
- Produce mobile and desktop wireframes showing section order, CTA hierarchy, navigation, Footer transitions, screenshots, and image-disabled states.
- Define each visual as `CSS/DOM`, `SVG illustration`, `product screenshot`, or `photographic asset`.
- Record source, creator, license, modifications, attribution, and redistribution rights for every non-original asset.
- Establish production-build performance and accessibility baselines for the current Home page.

### Verify

- Every available-now claim maps to a working route and replayed result.
- Every future claim is unmistakably labelled and has no misleading active CTA.
- The pages remain understandable as a text-only outline.
- No unresolved decision changes route structure, copy meaning, asset licensing, or release thresholds.

## Checkpoint 2: Routes, navigation, shell, and metadata

**Files:** `src/main.rs`, `src/pages/mod.rs`, `src/pages/home.rs`, new `src/pages/about.rs`, new `src/pages/how_to.rs`, `src/components/navbar.rs`, `src/components/footer.rs`, `src/components/app_layout.rs`, `assets/main.css`

### Tasks

- Add failing route/SSR tests for `/about`, `/how-to`, metadata, and catch-all ordering.
- Add typed routes before the catch-all and export both page components.
- Add Home, About, and How-to links to applicable desktop, mobile/drawer, and Footer navigation.
- Add current-route indication and ensure mobile menus close after navigation.
- Introduce a marketing page wrapper that permits full-width visual sections without changing application-page widths.
- Implement complete semantic page skeletons and typed route metadata.

### Verify

- Direct requests and client navigation render all three pages in server and web builds.
- SSR output contains page metadata, primary copy, headings, and working links.
- Navigation does not wrap, overflow, obscure content, or break the catch-all route.
- The pages remain usable without client JavaScript.

## Checkpoint 3: Shared marketing components and Home

**Files:** `src/pages/home.rs`, new `src/components/marketing/`, `src/components/mod.rs`, approved assets, `assets/main.css`

### Tasks

- Add tests for pure auth-aware CTA selection and feature availability mapping.
- Build only the reusable primitives demonstrated by at least two sections: marketing CTA, feature section, status badge, media frame, and final CTA panel.
- Replace inline Home styles with semantic classes and existing tokens.
- Build the approved static hero composition using DOM/CSS/SVG or an optimized image.
- Implement the value strip, feature stories, differentiator, community section, and final CTA.
- Add responsive states with logical source order and no horizontal overflow.

### Verify

- Signed-out and authenticated CTA labels/destinations are correct in SSR and after hydration.
- The page remains coherent with images disabled and at mobile, tablet, and desktop widths.
- Light/dark screenshots match approved wireframes.
- No production renderer, canvas, or continuous animation is present.

## Checkpoint 4: Traditional About page

**Files:** `src/pages/about.rs`, shared marketing components, approved About illustrations/screenshots, `assets/main.css`

### Tasks

- Implement the six-section editorial narrative in ordinary document flow.
- Use alternating text/visual layouts on wide screens and text-first stacked layouts on mobile.
- Add optional section anchors and a compact in-page contents list where useful.
- Use static compositions to connect kitchen landmarks with Capture, Organize, Evolve, Share, and Begin.
- Keep future-facing material in a visually and semantically distinct section.
- Limit motion to optional CSS transitions that disable under reduced motion.

### Verify

- Keyboard, touch, anchors, browser restoration, and native scrolling reach all six sections without special handling.
- Removing all visual assets leaves the complete story and final CTA.
- The page has no sticky region that obscures content or consumes most of a mobile viewport.
- Light/dark, reduced-motion, 200% zoom, and forced-color presentations remain usable.

## Checkpoint 5: How-to instructional page

**Files:** `src/pages/how_to.rs`, new `src/components/marketing/step_guide.rs`, assets under `assets/how-to/`, `assets/main.css`

### Tasks

- Create a task matrix with authenticated state, starting route, fixture, prerequisites, steps, expected result, and final route.
- Cover sign-in, recipe creation, editing, images, visibility, scaling, drafts/version history, collections, Dashboard/Explore discovery, sharing, and profiles only where verified.
- Add a table of contents and stable anchor IDs.
- Use fictional data in screenshots and document the screenshot refresh process.
- Add verified privacy callouts and troubleshooting for sign-in, uploads, visibility, and locating saved work.

### Verify

- A new user can replay every procedure without undocumented steps.
- Anchors and route links work with keyboard and client navigation.
- Screenshots match current UI and have useful alt text.
- Search instructions are omitted until the canonical NOMS-012 route exists.

## Checkpoint 6: Custom icon family and scoped migration

**Files:** new `assets/icons/README.md`, `assets/icons/source/*.svg`, generated `assets/icons/noms-icons.svg`, new `src/components/base/icon.rs`, `src/components/base/mod.rs`, public-page/navigation call sites, `assets/main.css`

### Tasks

- Inventory visible emoji, Unicode controls, provider marks, favicon, and CSS-drawn icons; mark each call site `NOMS-013`, `third-party`, `text`, or `follow-up`.
- Approve a contact sheet at 16, 20, 24, 32, and 48px in both themes before implementing the full core set.
- Use a 24×24 grid, rounded strokes/joins, `currentColor`, documented optical padding, and optional token-driven two-tone layers.
- Build a typed `Icon`/`IconName` API with decorative and accessible modes.
- Add deterministic optimization and completeness checks.
- Replace public-page/Navbar/Footer placeholder marks, menu/close/theme controls, and all icons introduced by NOMS-013.
- Keep OAuth/provider artwork unchanged.

### Verify

- Enum-to-symbol completeness and accessible/decorative behavior are tested.
- Icons survive SSR, hydration, theme changes, and client navigation.
- Icon-only controls have names, visible focus, and 44px targets.
- The scoped Unicode/emoji audit has only documented exclusions.

## Checkpoint 7: Production release gate

**Files:** page/component/browser tests, `docs/manual-test-guide.md`, approved visual baselines, design/asset manifests

### Automated validation

- Test Home, About, and How-to rendering, metadata, auth-aware CTAs, navigation, anchors, and icon accessibility.
- Add or select browser/a11y coverage for keyboard navigation, reduced motion, responsive layouts, theme changes, and image failures.
- Run `cargo fmt --check`, both WASM and server clippy/check commands, server-feature tests, the Tailwind production build, the selected browser test command, `dx bundle --platform web --release`, and `git diff --check`.
- Run an axe-style audit on all three pages in both themes and manually inspect landmarks and reading order.

### Manual matrix

- Current Chrome, Firefox, and Safari desktop.
- Current Chrome Android and Safari iOS in portrait and landscape.
- Signed-in/signed-out, light/dark, reduced motion, 200% zoom, keyboard only, forced colors, slow network, images disabled, and client navigation loops.

### Release thresholds

- No critical accessibility issue, keyboard trap, misleading feature claim, console error, or broken route.
- All content and CTAs work without images and without client JavaScript.
- No avoidable layout shift from media loading.
- Production performance meets recorded page and asset budgets or the responsible visual is simplified.
- No renderer dependency, canvas scene, render loop, GLB asset, or experimental import appears in the production bundle.
- Final copy receives a product-truth review.

## Checkpoint 8: Isolated 3D feasibility experiment

**Files:** new `experiments/NOMS-013-3d/`, new `docs/design/NOMS-013/3d-experiment.md`; no production page/component files

### Purpose

Determine whether a future progressive 3D enhancement would add enough value to justify its transfer size, runtime cost, accessibility burden, and maintenance complexity. This checkpoint produces evidence and a decision—not a production feature.

### Prototype contract

- Recreate one approved static About visual as a small representative 3D scene containing a camera, lighting, a few materials, a compressed glTF/GLB asset, theme adaptation, and one deterministic transition.
- Compare at least:
  1. Three.js `WebGLRenderer` or `WebGPURenderer` with fallback.
  2. One Rust/WASM approach: `three-d` or a minimal `wgpu` renderer.
- Use equivalent scene content, output dimensions, DPR, lighting intent, and measurement conditions.
- Keep prototypes independently buildable and outside the production dependency graph.
- Do not add Bevy/Fyrox unless the representative scene demonstrably requires engine-level animation/ECS features.

### Measurements

- Compressed JS/WASM runtime transfer size and total scene transfer size.
- Cold time to first poster and first stable rendered frame.
- Median CPU/GPU frame time over a documented 20-second capture.
- Idle CPU use when nothing changes.
- Peak memory and retained memory after ten mount/dispose cycles.
- WebGPU/WebGL/browser/device coverage and fallback behavior.
- Implementation size, build complexity, glTF fidelity, teardown complexity, and accessibility implications.

### Decision rule

The report selects one of three outcomes:

- **Do not pursue:** the static production design communicates the same value with materially lower cost.
- **Prototype further:** results are promising but browser, asset, lifecycle, or accessibility questions remain.
- **Propose follow-up:** one approach meets agreed budgets and provides a clear user-facing improvement.

Even a “Propose follow-up” result only creates a new roadmap issue with its own implementation plan, fallback contract, review, and release gate. It does not modify NOMS-013 production pages.

### Verify

- Production builds and tests pass with the entire experiment directory absent.
- Results are reproducible from documented commands and test conditions.
- The decision report includes raw measurements, limitations, rejected alternatives, recommendation, and proposed follow-up scope.

---

## Definition of done

- Home, About, and How-to are polished conventional public pages with accurate SSR content, responsive layouts, metadata, and working navigation.
- Home communicates Noms’ value and provides clear auth-aware actions.
- About tells the six-part product story through accessible editorial sections and static visuals.
- How-to documents only replayed workflows with stable anchors and canonical links.
- The scoped custom icon family is typed, themeable, accessible, optimized, and free of external runtime dependencies.
- Accessibility, mobile, SEO, theme, performance, and cross-browser release gates pass.
- The production bundle contains no 3D renderer or experimental asset.
- The isolated 3D experiment is documented and results in an explicit no-go, further-prototype, or follow-up decision.

## Recommended implementation order

Checkpoint 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8.

Icon concept work may begin alongside Checkpoint 1, but migration waits for contact-sheet approval. Home, About, and How-to implementation may overlap only after shared components and copy are stable. The 3D experiment begins after the traditional production experience passes Checkpoint 7.
