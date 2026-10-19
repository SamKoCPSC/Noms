# Kitchen journal landing-page validation

Implements the approved proposal 6 revision in the public `/` route using Rust
Dioxus components and route-scoped CSS. The shared app navigation, theme toggle,
fonts, footer, login return path, and authenticated dashboard/create actions stay
available. The landing page includes the five approved descriptions, eighteen
feature cards, the mission photo, mobile experience overview, and free/privacy
commitments.

The local WebP illustrations total about 1.1 MB. App captures use generated food
photographs and the synthetic example cook Robert McRobertson; the production
page does not seed or query these example recipes. Screenshot viewing uses the
existing native shared modal, including Escape, focus trapping/restoration, and
scroll locking. Cooking motion uses three real app frames, a pause control, and
a static reduced-motion fallback.

## Browser evidence

These captures come from the running implementation, including the real app
shell. Full-page captures extend from navigation through the footer. All were
visually reviewed for spacing, clipping, readable content, imagery, and theme
consistency. Section captures provide a closer look at the calculator/checklist,
feature cards, and mobile overview.

| View | Capture |
| --- | --- |
| Desktop, 1440 px | [Hero](desktop-hero.jpg) · [Full page](desktop-full.jpg) |
| Mobile, 393 px | [Hero](mobile-hero.jpg) · [Full page](mobile-full.jpg) |
| Dark theme | [Desktop full page](desktop-dark.jpg) · [Mobile full page](mobile-dark.jpg) |
| Calculator and checklist | [Feature story 3](calculator-checklist.jpg) |
| All eighteen feature cards | [Feature grid](feature-grid.jpg) |
| Mobile experience | [Three phone examples](mobile-experience.jpg) |

[Browser verification results](verification.json) cover widths 320, 393, 768,
and 1440 px, loaded images, overflow, all eight screenshot dialogs, Escape and
focus restoration, animation pause/play, normal and reduced motion, dark theme,
server-rendered content without JavaScript, the signed-out recipe-creation
return path, and public Explore navigation. No browser page or HTTP errors were
observed. This is Chromium emulation, not physical-device testing.

Reproduce with Python Playwright and an installed Chromium after starting the
local app as described in `docker/codex-cloud/README.md`:

```bash
python scripts/validate-landing.py \
  --url http://127.0.0.1:YOUR_APP_PORT \
  --output /tmp/noms-landing-review
```

Use the app URL printed by your stack. The script preserves the inherited proxy
for external traffic and excludes only loopback. Its output directory should be
outside the checkout. The page's feature illustrations are static examples;
clicking them opens a full-size screenshot, rather than controlling the app
shown inside the illustration.

## Rust checks

- `cargo check --locked --target wasm32-unknown-unknown`
- `SQLX_OFFLINE=true cargo check --locked --no-default-features --features server`
- `cargo clippy --locked --target wasm32-unknown-unknown -- -D warnings`
- `SQLX_OFFLINE=true cargo clippy --locked --no-default-features --features server -- -D warnings`
- `cargo fmt --check`
- Focused server-feature tests: `marketing_contract_tests` (7) and `components::` (182), using `SQLX_OFFLINE=true CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked --no-default-features --features server FILTER -- --test-threads=8`.
- `bash scripts/check-tracked-secrets.sh` and `git diff --check`.

The database integration suite was outside this presentation-only change's local
validation scope; the repository's normal PR workflow remains responsible for
its full CI suite.
