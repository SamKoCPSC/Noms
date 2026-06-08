# NOMS-007: Minor UI Fixes and Routing Cleanups

**Type:** Task / Bugfix
**Priority:** Low
**Status:** In Progress
**Created:** 2026-06-06

---

## Description

Address minor UI, routing, access-control, and styling issues
identified during systematic code review and manual testing.
Includes 404 handling, public explore page, profile avatar,
dropdown positioning, navbar glassmorphism, neumorphic depth,
container width, and unauthorized page styling.

---

## Sub-tasks

### [x] TC-36: Add 404 catch-all route

No branded 404 page existed. Unknown routes returned a plain
framework default.

**Fix:** Created `src/pages/not_found.rs` with themed UI, wired
`#[route("/:..segments")]` catch-all in `src/main.rs`, added
`.not-found` CSS class.

**Files:**
- `src/pages/not_found.rs` (new)
- `src/pages/mod.rs`
- `src/main.rs`
- `assets/main.css`

### [x] not_found.rs: Replace inline CSS with class

Component used inline styles on every element while a `.not-found`
CSS class existed but was never applied.

**Fix:** Replaced nested div wrapper with single
`div.not-found.container`, removed inline styles covered by CSS
class.

**Files:**
- `src/pages/not_found.rs`

### [x] TC-07: Add avatar to profile settings page

Avatar renders in navbar but not on `/settings/profile` page itself.
User has no visual identity cue on the page where they edit their
profile.

**Fix:** Added centered `Avatar` component (Large, 64px) at the top
of the profile form, above the display name field. Uses
`auth.current_user.avatar_url` for image source, falls back to
initials from `display_name` signal (with `username` as secondary
fallback when display name is empty). Shows "Provided by your OAuth
provider" hint text.

**Files:**
- `src/pages/settings/settings_profile.rs`

### [x] Explore page: Make public (unauthenticated access)

Explore page was wrapped in AuthRequired, blocking unauthenticated
users. This was intentional as a placeholder, but the page is meant
to be public for future community recipe browsing.

**Fix:** Removed AuthRequired wrapper and unused import from
`src/pages/explore.rs`. Page now renders directly inside AppLayout.

**Files:**
- `src/pages/explore.rs`

### [x] Navbar dropdown: Position beneath avatar trigger

Dropdown menu appeared offset from the avatar trigger, floating
away from the user menu area.

**Fix:** Added `.navbar-user-menu { position: relative }` as the
positioning container, changed dropdown from absolute offsets
(`top: 60px; right: var(--space-md)`) to `top: calc(100% + var(--space-xs));
right: 0` so it anchors directly beneath the trigger.

**Files:**
- `assets/main.css`

### [x] Navbar glassmorphism: Fix backdrop-filter stripped by CSS minifier

The frosted glass blur effect on the navbar was not rendering.
Dioxus uses Lightning CSS for minification, which drops
`backdrop-filter` when `-webkit-backdrop-filter` is present
(seeing them as duplicates). Chrome and Firefox ignore the
`-webkit-` prefixed version, so the blur was lost entirely.

**Fix:** Disabled CSS minification via
`CssAssetOptions::new().with_minify(false)` on both CSS assets
in `src/main.rs`. Removed JS injection workaround from
`navbar.rs`. Added `--glass-blur` CSS custom property to `:root`
and `.dark` scopes. Increased navbar background opacity to
`0.65` (light) and `0.75` (dark) for visible blur effect.

**Files:**
- `src/main.rs`
- `src/components/navbar.rs`
- `assets/main.css`

### [ ] Restyle buttons and text fields: Increase neumorphic depth

Buttons and text inputs use a subtle neumorphic (soft shadow)
style that lacks visual depth. The raised/inset effect is too
flat to clearly convey interactive state.

**Plan:** Increase shadow contrast and layering on `.btn`,
`.btn--primary`, `.btn--secondary`, `.btn--danger`, and input
elements to make the depth more apparent.

**Files:**
- `assets/main.css`

### [ ] Increase UI container width for 16:9 screens

The main content container is constrained to a narrow max-width
that leaves large empty margins on typical 16:9 displays.

**Plan:** Increase the `max-width` on `.container` (and any
related layout constraints) so content stretches to fill more
of the screen width on widescreen resolutions.

**Files:**
- `assets/main.css`

### [ ] Restyle buttons on unauthorized page

The unauthorized/forbidden page has buttons that don't match
the current button styling or lack proper visual hierarchy.

**Plan:** Update button styles on the unauthorized page to match
the restyled button components.

**Files:**
- `assets/main.css`
- Unauthorized page component (TBD)

### [x] Add left sidebar navigation drawer

No persistent navigation sidebar existed for quick access to all
pages. Users must rely on the top navbar links.

**Fix:** Added ☰ toggle button to navbar (visible 768px+) that opens
a left sidebar drawer with page links: Home, Dashboard, Explore,
New Recipe, Collections. Shows Settings link when authenticated.
Drawer slides in from left with fade overlay, closes via ✕ button,
backdrop click, or link navigation.

**Files:**
- `src/components/navbar.rs`
- `assets/main.css`

### [ ] Hamburger menu drawer: Fix transparent background

The mobile hamburger menu drawer has a completely transparent
background, making it unreadable and visually broken.

**Plan:** Add proper background color (likely `var(--surface)` or
`var(--bg-base)`) to the drawer container so content is visible.

**Files:**
- `assets/main.css`

---

## Acceptance Criteria

- [ ] All sub-tasks verified against their respective test case in
  `docs/manual-test-guide.md`
- [ ] No regressions in existing test suite
- [ ] Builds pass on both `server` and `web` targets
