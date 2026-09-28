# NOMS web logo

## Second painted version

[`noms-logo-painted-s3-v2.png`](noms-logo-painted-s3-v2.png) is the perspective-refinement variant generated with the built-in image tool on 2026-09-27 from v1. It retains the olive cover, cream NOMS plaque, coral closure/bookmark, slight tilt and painted finish, with more consistent alignment of the cover, plaque and page block. Selected as the production book logo on 2026-09-27; it replaces v1 in all shared BrandMark placements (navbar, footer and sign-in). V1 remains available as a historical source.

Generation brief: refine the existing logo into one coherent rigid book volume in a subtle three-quarter view; align the cover, plaque, lettering, page edges and elastic to a consistent perspective, reduce exaggerated corner depth, and preserve the original styling and transparent background.

## Current production version

The production painted PNG logo is `noms-logo-painted-s3-v2.png` (1122 × 1402 RGBA). The brief above records v2's perspective refinement. Open `compare.html` for a historical v1/SVG comparison on light/dark surfaces; [v1 generation notes](noms-logo-painted-s3-v1.md) remain provenance, not the current production asset. It retains genuine transparency and richer brushwork, but uses generated lettering rather than exact Fredoka outlines. Standardized 2026-09-27: navbar, footer and sign-in all use this same painted book through BrandMark; only the display size varies. The SVG is retained as an unused vector alternative. Shared Rust components preserve decorative accessibility treatment and aspect ratio; the asset pipeline emits a 192 × 240 transparent WebP shared by all book-logo placements.

`noms-logo.svg` is the static vector interpretation of **S3 — Continuous olive**, created 2026-09-27. The selected source is the rightmost book in [the spine comparison](../../roadmap/assets/NOMS-020/logo/spine-options-s1-s3-v1.png).

It preserves the clockwise tilt, continuous olive spine and cover, cream plaque/pages, coral elastic closure/bookmark, and warm directional lighting. Gradients and restrained facets simplify the raster concept's painted texture. It is a vector interpretation, not an exact trace of the painting.

The transparent SVG has a `512 × 640` viewBox and contains only vector geometry, gradients and a clip path. There are no embedded bitmaps, animation, scripts, filters or external dependencies. Its NOMS lettering is outlined from the app's **Fredoka**, weight 600, width 100; no font request is needed. Font source: [Google Fonts Fredoka](https://github.com/google/fonts/tree/main/ofl/fredoka), Copyright 2016 The Fredoka Project Authors, SIL Open Font License 1.1. The outlined artwork is a document created using that font.

## Web use

The shared `NavigationArtwork` component also places the v2 book in the navigation drawer heading and dashboard workspace heading/All recipes link. Its optional page-icon kind supplies matching painted destination artwork in a 32px navigation well, without changing page-heading sizes. These images are decorative beside existing labels.

Use the shared `BrandMark {}` component for navbar/footer and `BrandMark { large: true }` for sign-in. It imports `noms-logo-painted-s3-v2.png` through the Dioxus image pipeline and emits a single shared 192 × 240 WebP asset. The default display is 32 × 40; large is 96 × 120 (80 × 100 on narrow screens). Set an explicit width and height in the 4:5 canvas ratio (for example 80 × 100) to reserve layout space. Use `alt="NOMS"` for a standalone brand image or `alt=""` when adjacent text already names NOMS. Give a logo-only home link its own accessible name.

Prefer an external `img` rather than repeating inline SVG markup, which would duplicate gradient and accessibility IDs. Keep the transparent margin and aspect ratio; do not crop or stretch. At navigation sizes the silhouette remains recognizable but the cover lettering is too small to serve as readable text. Pair it with the app wordmark. A simplified tiny favicon remains a separate task.

Open `preview.html` to review 32, 48, 64, 96 and 320px heights on light and dark surfaces. The SVG is not used by production BrandMark; all placements use the painted PNG source. Favicon exports and the 3D model remain pending.

## Validation

V2 replacement (2026-09-27): developed in `codex/NOMS-020-logo-v2` / `.worktrees/NOMS-020-logo-v2`. The rendered shared-logo regression failed against v1 before the swap and passed afterward. `SQLX_OFFLINE=true cargo test --features server brand_ -- --nocapture` passed all 6 matching tests, including shared source/size/decorative accessibility and sign-in rendering. `cargo fmt --all --check` and `git diff --check` passed. Second-pass self-review found no outstanding issue in this scoped asset replacement; no independent-agent review was performed. The supplied PNG was copied byte-for-byte and v1 was retained. Live verification could not run: the existing app at port 32768 refused the browser connection and the configured Rancher Desktop Docker socket was absent. No substitute deployment or backend data changes were made.

The v2 swap also passed `SQLX_OFFLINE=true cargo check --target wasm32-unknown-unknown`. One cohesive commit carries the supplied asset, shared source change, regression expectations and production notes because these form a single asset-version replacement.

Historical SVG validation:

- Parsed as XML; all internal references resolve and IDs are unique.
- Checked absence of embedded raster images, live text, scripts, animation and external references.
- Rendered with `rsvg-convert assets/brand/noms-logo.svg -o /tmp/noms-logo-s3-preview.png`.
- Browser review uses `python3 -m http.server 32820 --bind 127.0.0.1 --directory assets/brand`, then opens `/preview.html`.
