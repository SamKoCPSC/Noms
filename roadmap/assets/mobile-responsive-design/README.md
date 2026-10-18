# Mobile responsive design — review evidence

Captured in the running app on 2026-10-17 using Chromium touch/viewport emulation,
local mock authentication and clearly identified demo recipes. These images show
the mobile implementation on main `3dd4e3e`, including the imported-draft
interaction correction and the revised navbar Search placement.

- [Navbar](navbar.png): Search and menu grouped on the right; 390px viewport.
- [Library](library.png): populated mobile library and bottom navigation.
- [Recipe](recipe.png): recipe reader on current main, including JSON export.
- [Current-main browser checks](current-main-browser-checks.json): six route
  families at 320px and 390px, actual JSON download/import into a private draft,
  and the mobile publication-review flow, with no JavaScript page errors.

The earlier review delivery contains 65 states across 24 route families and 47
full-page PNGs. Those captures predate the current-main integration; the three
images here provide a small repository-owned set for PR review.

The navbar was additionally checked at 320, 390, 430, 599, 600 and 760px, with an
8px Search-to-menu gap, centered buttons, and minimum 44px touch targets. Light
and dark appearance, Search navigation, focused editor headings and the desktop
header passed. Draft Back/save, section focus and theme persistence were also
verified in the browser.

Focused native UI tests use the server feature, with 182 component tests and 324
page tests passing after integration. The page run excludes
`avatar_media_ownership_tests`, whose PostgreSQL OS-user fixture is unavailable in
the development image. This is not a full database test result. Formatting,
whitespace and JavaScript syntax checks passed.

These checks do not establish physical-device keyboard/safe-area behavior,
assistive-technology compatibility or cross-browser support. The viewport helper
supports visual-keyboard changes, but that behavior still needs device testing.

## Rust viewport follow-up

The mobile viewport helper now uses typed `web_sys` APIs in
`src/utils/mobile_viewport.rs` and `src/utils/mobile_viewport/browser.rs`.
The standalone JavaScript asset and its dynamic import have been removed.
A scope-owned Rust registration removes listeners, cancels queued animation
frames and clears viewport CSS state when the layout unmounts.

The client/server build and three new Cargo tests passed.
[Runtime browser checks](web-sys-runtime-checks.json) verify all six editor-section
focus targets, a simulated 300px keyboard offset, fixed-action positioning,
blur/reset behavior, bottom-tab hiding/restoration, desktop behavior and browsers
without `visualViewport`. No handwritten mobile-runtime asset was requested and
no JavaScript page errors occurred. An isolated WASM harness built from the exact
helper source verified that unmounting cancels both pending frames and removes
all six listeners; repeated mounts do not accumulate listeners, and 50 resize
events coalesce into one update. These checks simulate viewport changes and do
not replace physical-device keyboard testing.
