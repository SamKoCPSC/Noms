# Develop Context

## Task Description
Implement Checkpoint 8 (Card Thumbnails + Polish): Recipe cards show the first image as a thumbnail with graceful fallback. Cards without images or failed image loads show a gradient placeholder. Lazy loading for viewport visibility. Error handling falls back to gradient. Alt text uses recipe title.

## Phase 0: Implementation Blueprint

# CP8 Card Thumbnails + Polish

## Research Findings

### Current State (Already Implemented)

The `RecipeCard` component at `src/components/base/recipe_card.rs` **already has the core conditional rendering logic** (lines 61-73):

- **Line 61**: `if let Some(first_image) = recipe.images.first()` — checks if recipe has images
- **Lines 62-67**: Renders `<img>` with `loading="lazy"`, `alt` set to `recipe.title`, `src` from first image URL
- **Lines 68-72**: Fallback `<span>` with "No image" text when no images exist
- **CSS** (`assets/main.css` lines 1028-1048): Already has `.recipe-card__image` with `aspect-ratio: 16/9`, `.recipe-card__image-img` with `object-fit: cover`, and `.recipe-card__image-placeholder`

### What Is Missing

1. **Gradient placeholder**: Current fallback is plain text "No image" — needs to be a gradient background
2. **Error handling**: No `onerror` handler on the `<img>` tag — broken/missing images cause a blank space
3. **Gradient CSS**: `.recipe-card__image-placeholder` lacks gradient styling

### Data Flow — Already Correct

- `Recipe` struct (`src/types.rs` line 47): `images: Vec<String>` — already available
- All three pages using `RecipeCard` pass the full `recipe` object (including images):
  - `src/pages/dashboard.rs` line 93: `RecipeCard { recipe: recipe.clone() }`
  - `src/pages/explore.rs` line 210: `RecipeCard { recipe: recipe.clone() }`
  - `src/pages/user_profile.rs` line 176: `RecipeCard { recipe: recipe.clone() }`
- No changes needed to any page — `RecipeCard` receives the full `Recipe` struct

---

## Implementation Plan

### Step 1: Add error handling state to RecipeCard

**File**: `src/components/base/recipe_card.rs`

Add a `use_signal` to track whether the image failed to load. On error, toggle from showing the `<img>` to showing the gradient placeholder.

**Changes** (inside `RecipeCard` function, after line 23):

```rust
// Track whether the image failed to load (for graceful fallback)
let mut image_failed = use_signal(|| false);
```

**Changes** (lines 61-73, replace the existing image block):

```rust
div {
    class: "recipe-card__image",
    if let Some(first_image) = recipe.images.first() {
        if !image_failed() {
            img {
                class: "recipe-card__image-img",
                src: first_image,
                alt: "{recipe.title}",
                loading: "lazy",
                onerror: move |_| {
                    image_failed.set(true);
                },
            }
        } else {
            div { class: "recipe-card__image-placeholder" }
        }
    } else {
        div { class: "recipe-card__image-placeholder" }
    }
}
```

**Key details**:
- `onerror` is a standard HTML event that Dioxus supports natively — when the browser fails to load the image, it fires and we set `image_failed = true`, triggering re-render with the gradient placeholder
- Changed fallback from `<span>` to `<div>` to properly fill the container with gradient
- The `move` closure captures `image_failed` by moving the signal into the handler
- Both the "no images" and "image failed" branches render the same `div.recipe-card__image-placeholder`

### Step 2: Add gradient placeholder CSS

**File**: `assets/main.css`

**Changes** (modify existing `.recipe-card__image-placeholder` at line 1038-1041):

Replace:
```css
.recipe-card__image-placeholder {
    font-size: 14px;
    color: var(--text-tertiary);
}
```

With:
```css
.recipe-card__image-placeholder {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
    color: var(--text-tertiary);
    background: linear-gradient(
        135deg,
        var(--bg-gradient-2) 0%,
        var(--bg-gradient-3) 50%,
        var(--bg-gradient-4) 100%
    );
}
```

**Rationale**:
- Uses existing CSS custom properties (`--bg-gradient-2`, `--bg-gradient-3`, `--bg-gradient-4`) defined in `:root` (lines 97-100) and `.dark` (lines 125-128) — ensures both light and dark mode work
- `width: 100%; height: 100%` ensures the gradient fills the 16:9 container
- `display: flex; align-items: center; justify-content: center` centers any text content (currently none, but keeps the structure consistent)
- The gradient uses the same warm, food-themed palette as the animated background gradient (lines 134-152)

### Step 3: Ensure `.recipe-card__image` container handles both states

**File**: `assets/main.css`

The existing `.recipe-card__image` (lines 1028-1036) already has the right properties:
- `width: 100%`
- `aspect-ratio: 16 / 9`
- `overflow: hidden`
- `display: flex; align-items: center; justify-content: center`

No changes needed — it already accommodates both the `<img>` (via `object-fit: cover`) and the gradient `<div>` (via `width/height: 100%`).

---

## Files Modified

| File | Lines | Change |
|------|-------|--------|
| `src/components/base/recipe_card.rs` | 24 (new), 61-73 (replace) | Add `image_failed` signal, `onerror` handler, gradient div fallback |
| `assets/main.css` | 1038-1041 (replace) | Gradient placeholder styling with existing CSS variables |

## Files NOT Modified (no changes needed)

| File | Reason |
|------|--------|
| `src/types.rs` | `Recipe.images: Vec<String>` already exists |
| `src/pages/dashboard.rs` | Already passes full `recipe` to `RecipeCard` |
| `src/pages/explore.rs` | Already passes full `recipe` to `RecipeCard` |
| `src/pages/user_profile.rs` | Already passes full `recipe` to `RecipeCard` |
| `src/components/base/mod.rs` | No new module needed |

---

## Verification Checklist

### Automated
- `cargo check` — compiles without errors or warnings
- `cargo check --features server` — compiles for server target (no wasm-specific code in changes)

### Manual
1. Navigate to dashboard → cards with uploaded images show thumbnails
2. Navigate to dashboard → cards without images show gradient placeholder (warm gradient, no "No image" text)
3. Simulate broken image URL (e.g., edit DB or mock a bad URL) → card falls back to gradient
4. Check dark mode → gradient uses dark-mode CSS variables
5. Verify `loading="lazy"` attribute is present on `<img>` tags (DevTools Elements panel)
6. Verify `alt` attribute equals recipe title on each card
7. Verify 16:9 aspect ratio is maintained for both image and gradient states
8. Test on mobile viewport → gradient fills container properly

---

## Architectural Notes

- **No new dependencies**: Uses existing Dioxus event handling (`onerror`) and CSS custom properties
- **No SSR concerns**: `use_signal` works fine in Dioxus full-stack; the `onerror` event only fires in the browser after hydration
- **Performance**: `loading="lazy"` is native browser lazy-loading (no IntersectionObserver needed); the gradient placeholder is pure CSS (no JS rendering cost)
- **Accessibility**: `alt` text on `<img>` uses recipe title; gradient placeholder has no text (intentional — the card title below provides context)
- **Future enhancement**: The DESIGN.md (line 1028) mentions blurhash-based gradient placeholders as a future optimization — this implementation uses static gradients as the baseline, which can be upgraded later without changing the component structure

## Phase 0.5: Blueprint Evaluation

### Verdict: **PASS**

The blueprint is well-researched, technically sound, and correctly addresses all requirements. All file paths, line numbers, function signatures, and CSS selectors were verified against the actual codebase and are accurate.

### Requirement Coverage

| Requirement | Status | Evidence |
|---|---|---|
| First image as thumbnail | ✅ Already implemented (line 61: `recipe.images.first()`) | No changes needed |
| Graceful fallback (no images) | ✅ Covered | `else` branch renders gradient placeholder |
| Error handling (broken images) | ✅ Covered | New `onerror` handler + `image_failed` signal |
| Gradient placeholder | ✅ Covered | CSS Step 2 adds `linear-gradient` with theme-aware vars |
| Lazy loading | ✅ Already implemented (line 66: `loading: "lazy"`) | No changes needed |
| Alt text = recipe title | ✅ Already implemented (line 65: `alt: "{recipe.title}"`) | No changes needed |

### Verified Claims (all confirmed against actual codebase)

1. **RecipeCard component** at `src/components/base/recipe_card.rs` — exists, 125 lines, structure matches description exactly
2. **Image rendering logic** at lines 61-73 — confirmed: `if let Some(first_image) = recipe.images.first()` with `img` and fallback `span`
3. **`Recipe.images: Vec<String>`** at `src/types.rs` line 47 — confirmed
4. **Three pages pass full `recipe`** — confirmed: `dashboard.rs:93`, `explore.rs:210`, `user_profile.rs:176` all use `RecipeCard { recipe: recipe.clone() }`
5. **CSS variables** `--bg-gradient-2`, `--bg-gradient-3`, `--bg-gradient-4` — confirmed in `:root` (lines 98-100) and `.dark` (lines 126-128)
6. **`.recipe-card__image`** container (lines 1028-1036) — confirmed: `width: 100%`, `aspect-ratio: 16/9`, `overflow: hidden`, `display: flex` with centering
7. **`.recipe-card__image-placeholder`** (lines 1038-1041) — confirmed: current styling is `font-size: 14px; color: var(--text-tertiary);`
8. **`use_signal`** usage — confirmed in `navbar.rs` (lines 14-16) and `image_upload.rs` (lines 485-489); `use dioxus::prelude::*` at `recipe_card.rs:9` includes it

### Technical Verification

**`onerror` handler — CONFIRMED WORKING:** Verified against Dioxus 0.7.9 `dioxus_html` source (`generated.rs` lines 108-113):
```rust
#[convert = convert_image_data]
#[events = [
    onerror => error,
    onload => load,
]]
Image(ImageData),
```
The `onerror` event is explicitly defined for `Image` elements, using `ImageData` as the event type. The blueprint's `move |_|` closure correctly ignores the event parameter.

**Gradient approach — SOUND:** Uses existing CSS custom properties that are already theme-aware (`:root` for light, `.dark` for dark mode). The gradient palette (`--bg-gradient-2` through `--bg-gradient-4`) is consistent with the animated background gradient used elsewhere (line 1726: `linear-gradient(135deg, var(--bg-gradient-2), var(--bg-gradient-3))`).

**Signal lifecycle — CORRECT:** `image_failed` is created via `use_signal` inside the component function. Each `RecipeCard` instance gets its own signal (pages create new instances per recipe via `RecipeCard { recipe: recipe.clone() }`). Signal persists for the card's lifetime and correctly toggles on error.

### Issues

**1. [SUGGESTION] No automated tests planned** — The verification checklist is entirely manual. Consider adding at least a unit test for the `image_failed` signal toggle logic (e.g., verify signal starts as `false` and can be set to `true`). The `Avatar` component has 10 unit tests for its helper function; `RecipeCard` could benefit from similar coverage for its new conditional rendering logic. **Risk: Low** — the logic is straightforward and the manual checklist is thorough.

**2. [SUGGESTION] Accessibility: gradient placeholder has no `aria-hidden`** — The gradient `<div>` is purely decorative but has no `aria-hidden="true"`. Screen readers may announce it as an empty region. Adding `aria_hidden: true` to the placeholder div would be a minor improvement. **Risk: Low** — the card title below provides context, and the placeholder has no text content.

**3. [SUGGESTION] `use_signal` placement could be more explicit** — The blueprint says "after line 23" which is the destructuring line. Being more specific (e.g., "add as the first line after destructuring, before the time calculation logic") would reduce ambiguity during implementation. **Risk: Negligible** — any placement before the `rsx!` block works correctly in Dioxus.

### Security, Performance, Maintainability

- **Security:** No concerns. No new data flows, no user input handling, no external dependencies.
- **Performance:** Optimal. Uses native browser `loading="lazy"` (no IntersectionObserver overhead), pure CSS gradient (zero JS cost), single signal per card (minimal memory).
- **Maintainability:** Clean separation of concerns — Rust handles state, CSS handles presentation. The gradient placeholder is easily themeable via existing CSS variables.

### Summary

The blueprint is thorough, accurate, and well-reasoned. It correctly identifies what already exists versus what needs to change, leverages existing infrastructure (CSS variables, Dioxus signals), and makes minimal, targeted modifications. The `onerror` approach is confirmed to work with Dioxus 0.7. All three identified issues are minor suggestions that do not affect correctness or functionality.

## Phase 1: Implementation Details

### Summary of Changes

Implemented Checkpoint 8: Card Thumbnails + Polish. Recipe cards now show the first image as a thumbnail with graceful fallback to a gradient placeholder when images are missing or fail to load. Added `onerror` handler for broken image URLs, `aria-hidden` on decorative placeholder for accessibility.

### Files Modified

| File | Change |
|------|--------|
| `src/components/base/recipe_card.rs` | Added `image_failed` signal, `onerror` handler on `<img>`, replaced `<span>` fallback with `<div class="recipe-card__image-placeholder" aria-hidden="true">` |
| `assets/main.css` | Replaced `.recipe-card__image-placeholder` with gradient background using `--bg-gradient-2/3/4` CSS custom properties, full-size flex container |

### Detailed Changes

**`src/components/base/recipe_card.rs`** (lines 26-27, 64-86):
- Added `let mut image_failed = use_signal(|| false);` after props destructuring to track per-card image load failures
- Added `onerror: move |_| { image_failed.set(true); }` on the `<img>` tag — triggers re-render replacing img with gradient div
- Changed the "no images" fallback from `<span class="...">No image</span>` to `<div class="recipe-card__image-placeholder" aria_hidden: true>`
- Both error and no-image branches now render the same gradient placeholder div

**`assets/main.css`** (lines 1038-1051):
- `.recipe-card__image-placeholder` now has `width: 100%; height: 100%` to fill the 16:9 container
- Added `display: flex; align-items: center; justify-content: center` for centering
- Added `background: linear-gradient(135deg, var(--bg-gradient-2) 0%, var(--bg-gradient-3) 50%, var(--bg-gradient-4) 100%)` — uses existing theme-aware CSS variables (light/dark mode)

### Verification

- `cargo check` — compiles cleanly with zero errors and zero warnings (wasm target)
- `cargo check --features server` — server target errors are all pre-existing sqlx database connection issues, unrelated to these changes
- Clippy errors are pre-existing in `recipe_edit.rs` and `recipe_new.rs` (needless `return`), not in `recipe_card.rs`
- No tests were written or updated — the logic is straightforward conditional rendering with a signal toggle; the blueprint evaluation noted this as low risk

## Phase 2: Review Verdict

### Verdict: **PASS**

### Numbered Issues

**1. [SUGGESTION] No automated tests for new signal logic** — The `image_failed` signal toggle and conditional rendering branch are untested. The logic is straightforward (one signal, two branches), so this is low risk. A unit test verifying the signal starts as `false` and the `else` branch renders the placeholder div would add confidence without much effort. **Location:** `src/components/base/recipe_card.rs` lines 27, 65-86.

### Positive Findings

1. **Correct `onerror` handler** — `onerror: move |_| { image_failed.set(true); }` at line 71-73 correctly uses `ImageData` event type (confirmed via Dioxus 0.7 docs), ignores the event parameter, and toggles the signal to trigger re-render. The `move` keyword properly captures the `Signal<bool>` with `'static` lifetime.

2. **`aria_hidden: true` on placeholder** — Both placeholder branches (error at line 78, no-image at line 84) include `aria_hidden: true`, correctly marking the decorative gradient div as non-interactive for screen readers. The `<img>` retains its `alt` text (`{recipe.title}`) when present.

3. **CSS gradient is theme-aware** — Uses existing `--bg-gradient-2/3/4` custom properties, verified present in both `:root` (light: `#F5E6D0`, `#E8EDE2`, `#F0DCC8`) and `.dark` (dark: `#221E18`, `#181E20`, `#1E1820`). The gradient palette is consistent with the animated background gradient used elsewhere in the app.

4. **16:9 container properly filled** — `.recipe-card__image-placeholder` has `width: 100%; height: 100%` and the parent `.recipe-card__image` enforces `aspect-ratio: 16 / 9`. The gradient fills the entire container in both states.

5. **No wasm leaks or server-only issues** — `cargo check` (wasm target) compiles cleanly with zero errors/warnings. `cargo check --features server` errors are all pre-existing sqlx database connection issues unrelated to these changes. No `#[cfg]` gates or platform-specific code introduced.

6. **No dead code or unused imports** — All four imports (`Avatar`, `AvatarSize`, `Recipe`, `dioxus::prelude::*`) are actively used. The `use_signal` from the prelude is the only new dependency and is correctly utilized.

7. **Clean conditional structure** — The nested `if let` + `if` pattern (lines 64-86) is readable and correctly handles three states: (a) image present and loaded, (b) image present but failed, (c) no images at all. States (b) and (c) share the same placeholder rendering.

### Requirements Coverage

| Requirement | Status | Evidence |
|---|---|---|
| First image as thumbnail | ✅ | Line 64: `recipe.images.first()`, line 68: `src: first_image` |
| Graceful fallback (no images) | ✅ | Lines 81-85: gradient placeholder div |
| Error handling (broken images) | ✅ | Lines 71-73: `onerror` handler toggles `image_failed` signal |
| Gradient placeholder | ✅ | CSS lines 1046-1051: `linear-gradient` with theme vars |
| Lazy loading | ✅ | Line 70: `loading: "lazy"` |
| Alt text = recipe title | ✅ | Line 69: `alt: "{recipe.title}"` |
| `aria-hidden` on placeholder | ✅ | Lines 78, 84: `aria_hidden: true` |

### Summary

Clean, minimal implementation that faithfully follows the Phase 0 blueprint. Two files changed, ~20 lines of new/modified code. The `onerror` + `use_signal` pattern is idiomatic Dioxus, the CSS leverages existing theme infrastructure, and accessibility considerations (`aria-hidden`, `alt` text) are addressed. One minor suggestion around test coverage; no blockers or warnings.

## Phase 3: Synthesis
## User-Facing Summary

Checkpoint 8 (Card Thumbnails + Polish) is complete. Recipe cards now display the first uploaded image as a thumbnail with `loading="lazy"` for performance. When a recipe has no images, or when an image fails to load (broken URL, missing file), the card gracefully falls back to a warm gradient placeholder instead of showing blank space or plain "No image" text. The gradient adapts to light and dark mode via existing CSS custom properties. All image elements use the recipe title as `alt` text for accessibility, and the decorative gradient placeholder is marked `aria-hidden` so screen readers skip it.

**Workflow**: Phase 0 identified that the core conditional rendering (`recipe.images.first()`) and lazy loading were already in place. Only two gaps remained: (1) no `onerror` handler for broken images, and (2) the fallback was plain text instead of a styled gradient. Phase 0.5 verified all line numbers, Dioxus event signatures, and CSS variable availability. Phase 1 implemented the `image_failed` signal, the `onerror` closure, and the gradient CSS. Phase 2 reviewed and passed the implementation with one minor suggestion around test coverage.

---

## Step-by-Step Walkthrough of Changes

### File 1: `src/components/base/recipe_card.rs`

**Purpose**: Add runtime error handling for image loads and replace the text fallback with an accessible gradient placeholder.

**Logic flow**:
1. A new `use_signal` (`image_failed`) is declared after props destructuring. Each `RecipeCard` instance gets its own independent signal initialized to `false`.
2. The existing `if let Some(first_image) = recipe.images.first()` block now has a nested `if !image_failed()` guard. When `image_failed` is `false`, the `<img>` tag renders as before.
3. A new `onerror` handler on the `<img>` tag uses a `move` closure to capture the `image_failed` signal. When the browser fires the native `error` event (e.g., 404, network failure), `image_failed.set(true)` triggers a Dioxus re-render.
4. On re-render, the `if !image_failed()` condition is now `false`, so the `else` branch renders `<div class="recipe-card__image-placeholder" aria-hidden="true">` — a gradient div instead of the `<img>`.
5. The original `else` branch (no images in the `Vec`) was changed from `<span>No image</span>` to the same `<div class="recipe-card__image-placeholder" aria-hidden="true">`, unifying both failure modes.

**Resulting state machine per card**:
- State A: `images` is empty → gradient placeholder
- State B: `images` has entries, `image_failed` is `false` → `<img>` with thumbnail
- State C: `images` has entries, `image_failed` is `true` (after `onerror`) → gradient placeholder

### File 2: `assets/main.css`

**Purpose**: Style the placeholder div as a visually appealing gradient that fills the 16:9 container and adapts to theme.

**Changes**:
- `.recipe-card__image-placeholder` gained `width: 100%; height: 100%` so it fills the parent `.recipe-card__image` (which enforces `aspect-ratio: 16 / 9`).
- `display: flex; align-items: center; justify-content: center` ensures any future text content would be centered.
- `background: linear-gradient(135deg, var(--bg-gradient-2) 0%, var(--bg-gradient-3) 50%, var(--bg-gradient-4) 100%)` — a 3-stop diagonal gradient using existing theme-aware CSS custom properties:
  - Light mode: `#F5E6D0` → `#E8EDE2` → `#F0DCC8` (warm, food-themed tones)
  - Dark mode: `#221E18` → `#181E20` → `#1E1820` (subtle dark variation)

---

## Dependencies

- **No new dependencies introduced**. All changes use existing infrastructure:
  - `use_signal` from `dioxus::prelude::*` (already imported at line 9)
  - `onerror` event on `<img>` (native Dioxus HTML binding, confirmed in Dioxus 0.7 `dioxus_html` source)
  - CSS custom properties `--bg-gradient-2/3/4` (already defined in `:root` and `.dark`)

---

## Special Syntax & Language Features

- **`move` closure in `onerror`**: The `move` keyword is required because Dioxus event handlers need `'static` lifetime. The `Signal<bool>` is `Clone` and internally `Arc`-based, so moving it into the closure is safe and idiomatic.
- **`aria_hidden: true`**: Dioxus converts snake_case prop names to HTML attributes (`aria-hidden`). This marks the decorative gradient div as non-semantic for assistive technologies.
- **Nested `if let` + `if` pattern**: The three-state conditional (no images / image OK / image failed) is expressed as `if let Some(...) { if !failed { img } else { placeholder } } else { placeholder }`. This avoids cloning or optionality overhead.
- **Native `loading="lazy"`**: Uses the browser's built-in lazy loading (no IntersectionObserver or JS library needed). The attribute is already present and unchanged.

---

## Follow-Up Recommendations

1. **Test coverage** (low priority): Add a unit test verifying the `image_failed` signal starts as `false` and the conditional rendering produces the correct DOM in each state. The `Avatar` component has 10 unit tests as a reference pattern.
2. **Blurhash placeholders** (future enhancement): DESIGN.md line 1028 mentions blurhash-based gradients. The current static gradient can be upgraded without changing the component structure — only the CSS background would need to be dynamic.
3. **Monitor image load failures**: If production cards frequently show gradients, investigate whether image URLs are being generated correctly or if CDN/storage issues exist.

---

## Commit Message

```
feat(NOMS-009): add onerror handler and gradient placeholder to recipe card thumbnails

RecipeCard now gracefully handles two failure modes for image thumbnails:
when a recipe has no images and when an image fails to load (broken URL,
missing file, network error). Both cases render a theme-aware gradient
placeholder instead of blank space or plain text.

Changes:
- recipe_card.rs: add `image_failed` signal (use_signal) to track per-card
  image load failures; add `onerror` handler on `<img>` that sets the
  signal to true, triggering re-render with the gradient fallback;
  replace `<span>No image</span>` with `<div>` placeholder; add
  `aria-hidden` to both placeholder branches for accessibility
- main.css: style `.recipe-card__image-placeholder` with a 3-stop
  diagonal gradient (135deg) using existing `--bg-gradient-2/3/4` CSS
  custom properties; set `width/height: 100%` and flex centering so
  the gradient fills the 16:9 container; gradient adapts to light and
  dark mode via existing theme variables

Existing features preserved:
- `loading="lazy"` on `<img>` for native browser lazy loading
- `alt` text set to recipe title for screen reader accessibility
- 16:9 aspect ratio maintained across all rendering states
- No new dependencies or breaking changes
```
