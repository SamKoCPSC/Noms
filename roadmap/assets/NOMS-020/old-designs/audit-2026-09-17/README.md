# App audit — 2026-09-17

Fresh browser captures of the running app, not redesign proposals. This is the preferred desktop baseline over the clipped original audit.

## Capture details

- Date: 2026-09-17 (America/Vancouver).
- Running checkout: `/home/samko/GitRepos/Noms`, revision `ce080b42f20f6c19b12fab1151c67fabf3e86491`.
- The checkout was not clean: `assets/tailwind.css` and three roadmap documents already had uncommitted changes. These captures represent that working checkout, not a pristine commit.
- App: existing local Docker deployment `noms-noms020-image-audit`, served at `http://127.0.0.1:32773/`.
- Browser: Codex in-app Chromium browser; light theme. Requested viewport 1920×1080; native browser screenshot capture region 1920×1080 at scale 1. The host reported browser zoom 0.8, so do not use these images as a pixel-perfect CSS sizing specification.
- All `*-desktop.png` files are actual **1920×1080 (16:9)** PNGs. They were not stretched, padded, or cropped in post-processing.
- `*-full-page.png` files are separate tall captures at 1920 pixels wide, intentionally **not 16:9**, to retain below-the-fold content.
- Chrome extension captures showed clipped/repeated tiles and were rejected. Saved captures use the in-app browser's native screenshot command with capture beyond the visible surface enabled.
- The obsolete original audit and before/after folders were subsequently removed at the user's request on 2026-09-17; this refreshed set is retained.

Stylesheet SHA-256 at capture:

```text
assets/main.css     3fb43672eec1fef4f89cc96c0915a61b1ea86b156b2d516f755a637fc90ca998
assets/tailwind.css 1271f9af9e64303481673498346c4460681464775625b5b650dd64d0e5c9fe8e
```

## Screenshot index

| Page/state | 16:9 desktop | Full page | Route/state |
| --- | --- | --- | --- |
| Home | [View](01-home-desktop.png) | [View](01-home-full-page.png) | `/` |
| About | [View](02-about-desktop.png) | [View](02-about-full-page.png) | `/about` |
| How to | [View](03-how-to-desktop.png) | [View](03-how-to-full-page.png) | `/how-to` |
| Search sign-in gate | [View](04-search-signed-out-desktop.png) | — | `/search?` |
| Sign in | [View](05-sign-in-desktop.png) | — | `/login` |
| Populated dashboard | [View](06-dashboard-desktop.png) | — | `/dashboard` |
| Sidebar open on dashboard | [View](07-sidebar-desktop.png) | — | `/dashboard` |
| Profile settings | [View](08-profile-settings-desktop.png) | [View](08-profile-settings-full-page.png) | `/settings/profile` |
| Linked accounts | [View](09-linked-accounts-desktop.png) | — | `/settings/accounts` |
| Empty collections | [View](10-collections-empty-desktop.png) | — | `/collections` |
| New collection | [View](11-new-collection-desktop.png) | — | `/collections/new` |
| Search results | [View](12-search-desktop.png) | [View](12-search-full-page.png) | `/search?` |
| Public profile | [View](13-public-profile-desktop.png) | — | `/u/user-f025` |
| Explore | [View](14-explore-desktop.png) | — | `/explore` |
| Recipe detail | [View](15-recipe-detail-desktop.png) | [View](15-recipe-detail-full-page.png) | `/recipes/1c570426-0d3b-4296-b0d8-4cab4010ce84` |
| Expanded version differences (scrolled) | [View](16-recipe-history-desktop.png) | [View](16-recipe-history-full-page.png) | `same recipe, View differences` |
| Version graph | [View](17-version-graph-desktop.png) | [View](17-version-graph-full-page.png) | `same recipe /version-graph` |
| Existing recipe editor | [View](18-recipe-editor-desktop.png) | [View](18-recipe-editor-full-page.png) | `same recipe /edit` |
| New recipe editor | [View](19-new-recipe-desktop.png) | [View](19-new-recipe-full-page.png) | `/recipes/new` |

## Fixture and scope notes

- Public screens were captured signed out; signed-in screens use the existing local mock-OAuth account `image-audit` / `user-f025`.
- Existing test content: Audit Tomato Soup (two published versions) and Audit Spicy Tomato Soup (one fork). These recipes have no photos, and the profile has no display name or bio. Blank image placeholders and sparse metadata reflect the actual fixture, not removed UI.
- Collections is genuinely empty for this account. No new collections or recipes were published for the audit.
- Opening the existing recipe editor may initialize its private edit draft; no fields were changed or published.
- The history desktop image is intentionally scrolled to show expanded differences. Other desktop captures show the top of their page/state.
- The initial refresh covers desktop light-mode pages and the shared sidebar; mobile and dark examples are added below. It does not reproduce an empty dashboard with another account or cover every menu, modal, error, image-upload state, or populated collection. Responsive discovery remains per page under the implementation plan.
- Screenshot dimensions were checked, and representative public, dashboard, graph, editor, and history captures were visually inspected for the original clipping/repeated-tile problem.

A 16:9 screenshot records one screen, not the whole length of a long page. Use the full-page companion when reviewing all sections.

## Mobile and dark-mode supplement

Added on the same date, against the same revision and unchanged Tailwind stylesheet hash. These 13 additional captures use the existing signed-in audit account, including Home (which therefore shows “Open dashboard”). They document current behavior, not approved responsive or dark-mode redesigns.

- Mobile: 390×844 portrait screenshots, in both themes. This is browser viewport testing, not physical-device or touch/keyboard testing.
- Desktop dark: 1920×1080 screenshots.
- All sizes were checked from the saved PNG headers. Representative mobile and desktop-dark captures were visually inspected.
- Theme was changed through the app controls and returned to light afterward; the temporary viewport override was reset.
- Existing limitations are preserved: for example, the mobile editor's timing row extends beyond the right edge. This is app layout overflow, not screenshot truncation. These examples do not establish that mobile usability is complete.

| Page/state | Mobile light | Mobile dark | Desktop dark |
| --- | --- | --- | --- |
| Home | [View](32-home-mobile-light.png) | [View](24-home-mobile-dark.png) | [View](25-home-desktop-dark.png) |
| Navigation open | [View](21-navigation-mobile-light.png) | [View](23-navigation-mobile-dark.png) | — |
| Explore | [View](31-explore-mobile-light.png) | [View](27-explore-mobile-dark.png) | [View](26-explore-desktop-dark.png) |
| Recipe detail | [View](30-recipe-detail-mobile-light.png) | [View](28-recipe-detail-mobile-dark.png) | [View](29-recipe-detail-desktop-dark.png) |
| New recipe editor | [View](20-new-recipe-mobile-light.png) | [View](22-new-recipe-mobile-dark.png) | — |
