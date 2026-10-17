# NOMS-022: Mobile Responsive Design

**Status:** 🟡 In Progress — implementation complete; PR review pending
**Phase:** User experience
**Recorded retroactively:** 2026-10-17
**Depends on:** Existing application routes and the [NOMS-020 visual direction](../implementation-plans/NOMS-020-visual-design-refinement.md)
**Pull request:** [#6](https://github.com/SamKoCPSC/Noms/pull/6)

## Objective

Give every existing Noms page a deliberate mobile layout that feels familiar to modern phone users while retaining the app's neumorphic components, painted artwork, colors, Fredoka headings and Nunito body text. Preserve the existing data, authentication, visibility and write contracts.

The mobile designs were reviewed as a combined HTML document, aligned with the existing components and fonts, and approved for implementation. This task records that work retroactively; opening the implementation PR does not mark it shipped.

## Implemented scope

- Shared mobile shell at widths up to 760px: route headings, guarded editor Back, Search beside the menu, safe-area spacing, thumb navigation for Home/Explore/Library/You, and focused reading/editing layouts.
- A mobile You hub for profile, settings, appearance, help and sign-out; persistent theme state and locally served licensed Fredoka/Nunito fonts.
- Welcome, About, How To, sign-in, Community Home, Explore, Search, fallback, public-profile and settings layouts.
- Library list/grid and Recipes/Collections/Drafts navigation, creation sheet, collections, nested albums and collection forms.
- Recipe readers, serving controls and ingredient checklists, collection selection, full-screen cooking, immutable versions, and timeline/map history navigation.
- Sectioned new/edit/draft editors with mounted form state, fixed actions, image editing, publication review and save/resume behavior; responsive recipe import.
- Integration with NOMS-016's JSON import/export and imported-image review after rebasing onto current main.
- Light/dark appearance, compact 320px layouts, focus targets, reduced-motion behavior and keyboard viewport positioning. Physical-device keyboard and safe-area behavior still require device testing.

## Acceptance criteria

- [x] Mobile layouts cover all 24 routed page families represented in the review capture set, including signed-out and not-found states.
- [x] Existing neumorphic styling, typography and artwork are retained; the finalized Home illustration remains in the drawer.
- [x] The mobile navbar groups Search and menu on the right with an 8px gap and touch targets of at least 44px.
- [x] Main mobile flows operate through real application handlers: uploads, publication, editing, collections, scaling, cooking, search, history and import.
- [x] Moving between editor sections retains state; mobile Back saves before leaving; publication requires explicit confirmation on mobile.
- [x] Theme preference survives reload with consistent colors and switch semantics.
- [x] Representative compact pages have no horizontal overflow; desktop navigation remains available.
- [x] Browser screenshots and a downloadable gallery/PDF were delivered for review.
- [ ] PR review and merge to main.

## Verification evidence

[Repository screenshots and verification notes](../assets/mobile-responsive-design/README.md) include the updated navbar, Library, recipe reader and current-main browser checks.

- 65 mobile screenshot states across 24 routes, plus 47 full-page PNGs, captured in Chromium using a local demo account. Main viewport: 390 × 844, device scale factor 2; compact viewport: 320 × 740. The original gallery records the pre-rebase review snapshot, with revised navbar captures provided separately.
- Navbar alignment, 8px spacing, 44px touch targets and overflow checked at 320, 390, 430, 599, 600 and 760px; light/dark, Search navigation, focused editor and desktop headers checked.
- Browser checks cover section-heading focus, draft save-before-exit, bottom navigation and theme persistence.
- All 506 focused UI tests passed (182 component tests and 324 page tests). Focused native component/page tests use `cargo test --locked --no-default-features --features server --bin noms` with the `components::` and `pages::` filters. Page tests exclude `avatar_media_ownership_tests`, which require separate PostgreSQL OS-user fixtures; this is not a full database suite result.
- Current-main browser checks at 320px and 390px cover six route families and a real JSON export/import round trip into an interactive private draft, followed by publication review. The imported-draft editor emits `inert` only while image review is pending, so hydration cannot disable the idle editor.
- Formatting and whitespace checks: `cargo fmt --all -- --check` and `git diff origin/main --check`.

## Limits and follow-ups

- Viewport emulation is not physical-device or cross-browser validation. Check actual iOS/Android keyboard behavior, safe areas and assistive technology before a mobile release.
- Existing anonymous Search access, Explore tag filtering, route metadata and broader regression automation remain [NOMS-021 production-readiness follow-ups](NOMS-021-production-readiness-follow-ups.md); this task does not claim to fix those contracts.
- An installable PWA, offline support, push notifications and native application packaging are outside this scope.
