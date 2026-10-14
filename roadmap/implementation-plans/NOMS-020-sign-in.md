# NOMS-020 — Sign-in implementation

## Scope and status

Implemented the finalized [L6B direction](../assets/NOMS-020/signin-l6b-guest-access-v4.md) on `codex/NOMS-020-sign-in`, based on `516103a1be0554fef55f4b38f5d8f2060b0d6d0e`. This page-owned checkpoint avoids modifying the shared plan during parallel page work.

- [x] Centered welcome heading and compact raised provider panel.
- [x] Equal Google/GitHub outlined buttons using the shared button styling and official, locally served provider marks.
- [x] Guest exploration link and paired guest/account feature panels; account features explicitly supplement guest access.
- [x] Visibility-qualified public/shared recipe and history copy.
- [x] Existing Fredoka/Nunito typography, coral accents, shared material and elevation tokens.
- [x] Phone stacking, tablet/desktop paired panels, and existing dark-theme regression check.
- [x] Preserve OAuth redirect handling, authenticated-user routing, and native navigation/retry behavior.
- [ ] Integrate the canonical S3 logo (selected 2026-09-27, superseding S1) when a production-ready asset is available. Do not substitute a superseded proposal or crop an exploration sheet; existing shared navbar branding remains.

Owned implementation: `src/pages/login.rs`, `assets/sign-in.css`, and `assets/sign-in/`. No shared CSS, navigation, authentication backend, or other page changes.

## Interaction decisions

Provider actions remain native OAuth links. Browser navigation supplies pending feedback without a persistent client-side disabled state; cancelling via Back leaves both choices usable. No unimplemented email/password or signup form is introduced.

Existing OAuth failures are handled by the authentication endpoints, not by a new login query-string error protocol. This change does not add error banners that those endpoints cannot populate. Real external-provider consent and failure responses were not exercised.

## Validation — 2026-09-24

Isolated instance: `signin-page-20260924`, login preview `http://127.0.0.1:32779/login` (ephemeral port).

- Dioxus development build: server and WASM client succeeded.
- `cargo fmt --check` and `git diff --check`: passed.
- `SQLX_OFFLINE=true cargo test --features server pages::login::tests -- --test-threads=4`: six passed. Tests cover rendered provider/guest links and feature copy, nested OAuth return-query encoding, and existing redirect normalization.
- Independent read-only review: passed, no blocking or should-fix findings.
- Server Clippy attempted but unavailable: the container toolchain does not include `cargo-clippy`.

Browser checks (actual CSS viewport sizes):

| Check | Result |
| --- | --- |
| Desktop 1440 × 1000 | Equal provider controls; equal paired feature panels; no horizontal overflow or footer overlap. |
| Portrait tablet 820 × 1180 | Paired panels fit; no horizontal overflow. |
| Phone 390 × 843 | Feature panels stack with a gap; equal provider controls; all content scrollable without footer overlap or horizontal overflow. |
| Keyboard | Google → GitHub → guest link tab order; visible shared focus outline. |
| Theme | Existing dark tokens applied to surfaces/text; no overflow; restored light theme. |
| Provider assets | Both local marks loaded successfully. |
| Guest action | Opens public Explore without signing in. |
| Google handoff/cancellation | Opens isolated mock OAuth; Back returns to a usable login page with return target retained. |
| GitHub completion | Isolated mock OAuth completes and returns to requested `/recipes/new`. |
| Authenticated login visit | Redirects to `/dashboard`; test account signed out afterward. |

Browser screenshot capture produced tiled/stitching artifacts, including through the alternative capture path. Responsive evidence therefore combines live UI/accessibility inspection with DOM geometry and computed styles, not a claimed clean screenshot baseline. Final pixel-level comparison should be repeated when capture is reliable. Viewport override was reset and the preview left on signed-out `/login`.
