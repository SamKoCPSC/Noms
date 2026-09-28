# NOMS web logo

A painted PNG alternative is available in `noms-logo-painted-s3-v1.png` (1122 × 1402 RGBA, 1.36 MB). Open `compare.html` for a side-by-side comparison with the SVG on light/dark surfaces. See [generation notes and prompt](noms-logo-painted-s3-v1.md). It retains genuine transparency and richer brushwork, but uses generated lettering rather than exact Fredoka outlines. Integrated 2026-09-27: the SVG accompanies the navbar/footer wordmark and the painted book appears above the sign-in heading. Shared Rust components preserve decorative accessibility treatment and aspect ratio; the asset pipeline emits a 192 × 240 transparent WebP for the painted placement.

`noms-logo.svg` is the static vector interpretation of **S3 — Continuous olive**, created 2026-09-27. The selected source is the rightmost book in [the spine comparison](../../roadmap/assets/NOMS-020/logo/spine-options-s1-s3-v1.png).

It preserves the clockwise tilt, continuous olive spine and cover, cream plaque/pages, coral elastic closure/bookmark, and warm directional lighting. Gradients and restrained facets simplify the raster concept's painted texture. It is a vector interpretation, not an exact trace of the painting.

The transparent SVG has a `512 × 640` viewBox and contains only vector geometry, gradients and a clip path. There are no embedded bitmaps, animation, scripts, filters or external dependencies. Its NOMS lettering is outlined from the app's **Fredoka**, weight 600, width 100; no font request is needed. Font source: [Google Fonts Fredoka](https://github.com/google/fonts/tree/main/ofl/fredoka), Copyright 2016 The Fredoka Project Authors, SIL Open Font License 1.1. The outlined artwork is a document created using that font.

## Web use

Import `asset!("/assets/brand/noms-logo.svg")` through the existing Dioxus asset pipeline. Set an explicit width and height in the 4:5 canvas ratio (for example 80 × 100) to reserve layout space. Use `alt="NOMS"` for a standalone brand image or `alt=""` when adjacent text already names NOMS. Give a logo-only home link its own accessible name.

Prefer an external `img` rather than repeating inline SVG markup, which would duplicate gradient and accessibility IDs. Keep the transparent margin and aspect ratio; do not crop or stretch. At navigation sizes the silhouette remains recognizable but the cover lettering is too small to serve as readable text. Pair it with the app wordmark. A simplified tiny favicon remains a separate task.

Open `preview.html` to review 32, 48, 64, 96 and 320px heights on light and dark surfaces. The SVG is now integrated through the shared BrandMark component. Favicon exports and the 3D model remain pending.

## Validation

- Parsed as XML; all internal references resolve and IDs are unique.
- Checked absence of embedded raster images, live text, scripts, animation and external references.
- Rendered with `rsvg-convert assets/brand/noms-logo.svg -o /tmp/noms-logo-s3-preview.png`.
- Browser review uses `python3 -m http.server 32820 --bind 127.0.0.1 --directory assets/brand`, then opens `/preview.html`.
