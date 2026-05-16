# NOMS-003: UI Scaffold & Application Shell

**Status:** ⚪ Backlog
**Phase:** 1 — Foundation
**Created:** 2026-05-15

## Description

Build the visual and navigational skeleton of the Noms application. This is purely frontend scaffolding — no backend infrastructure, no database queries, no auth logic. Every subsequent Phase 1 issue fills into this shell.

The current codebase still carries the default `dx init` template (Blog page, Echo component, Hero component, dark-space-themed CSS). Replace all of that with Noms' actual route architecture, layout components, and design system foundations as specified in [DESIGN.md § UI/UX Design Principles](/DESIGN.md#uiux-design-principles).

## Scope

### 1. Route Architecture

Replace the template routes (`/`, `/blog/:id`) with Noms' real page structure. All pages render as empty shells or placeholder content — no data fetching, no forms wired up.

| Route | Component | Purpose |
|-------|-----------|---------|
| `/` | `Home` | Landing page / hero with app branding |
| `/login` | `Login` | Sign-in page with OAuth provider buttons (not wired yet) |
| `/dashboard` | `Dashboard` | User's recipe library (empty state) |
| `/recipes/new` | `RecipeNew` | Create recipe page (empty form shell) |
| `/recipes/:id` | `RecipeDetail` | Single recipe view (placeholder) |
| `/collections` | `CollectionList` | Collection browser (placeholder) |
| `/collections/:id` | `CollectionDetail` | Single collection view (placeholder) |
| `/explore` | `Explore` | Public recipe discovery (placeholder) |
| `/settings/profile` | `SettingsProfile` | User profile settings (placeholder) |
| `/settings/accounts` | `SettingsAccounts` | Linked OAuth accounts (placeholder) |

**Routing structure:**
```
Route enum
├── #[layout(AppLayout)]      — Shared shell (navbar + content area)
│   ├── Home {}               — /
│   ├── Login {}              — /login
│   ├── Dashboard {}          — /dashboard
│   ├── RecipeNew {}          — /recipes/new
│   ├── RecipeDetail { id }   — /recipes/:id
│   ├── CollectionList {}     — /collections
│   ├── CollectionDetail { id } — /collections/:id
│   ├── Explore {}            — /explore
│   └── Settings              — Nested layout for settings pages
│       ├── SettingsProfile {}   — /settings/profile
│       └── SettingsAccounts {}  — /settings/accounts
```

### 2. Layout Components

#### AppLayout
- Wraps all routes with a shared shell
- Composition: `Navbar` → `Outlet` (page content)
- Full-height flex layout so footer stays at bottom

#### Navbar
- Fixed/sticky at top with glassmorphic styling (`backdrop-filter: blur(12px)`, translucent fill)
- Logo/brand link (left)
- Primary navigation links (center): Dashboard, Explore, New Recipe
- Auth-aware slot (right): placeholder avatar + username when signed in, "Sign In" button when not
- Theme toggle button (right, sun/moon icon) — wired to `use_theme` hook
- Not wired to real auth — uses a hardcoded `Option<User>` signal that can be toggled for visual testing
- **Responsive behavior:** On screens < 768px, nav links collapse into a hamburger menu with an animated slide-out drawer

#### Footer
- Minimal footer with copyright and "Built with Dioxus" credit

### 3. Design System Foundations

Implement the visual design specified in DESIGN.md § UI/UX Design Principles. This is not pixel-perfect polish — it's the structural CSS variables and utility classes so every subsequent component has a palette to reference.

#### CSS Variables — Light & Dark Mode

Define both palettes in `main.css`. Every component references tokens only — no hardcoded hex values.

**Light mode** on `:root`:
```css
:root {
    /* Backgrounds */
    --bg-base: #FAF7F2;
    --surface: #F5F0E8;

    /* Neumorphic shadows */
    --shadow-light: #FFFFFF;
    --shadow-dark: #DDD8CE;

    /* Glassmorphism */
    --glass-fill: rgba(255, 255, 255, 0.20);
    --glass-border: rgba(255, 255, 255, 0.30);

    /* Accent */
    --accent: #D9735A;
    --accent-hover: #C4613F;
    --success: #5A9E6F;
    --warning: #D4923B;
    --error: #C4504A;

    /* Text */
    --text-primary: #2D2A26;
    --text-secondary: #7A756D;
    --text-tertiary: #A8A29A;

    /* Typography */
    --font-body: 'Nunito', 'Segoe UI', system-ui, sans-serif;
    --font-display: 'Fredoka', 'Georgia', serif;
}
```

**Dark mode** on `[data-theme="dark"]`:
```css
[data-theme="dark"] {
    --bg-base: #1E1C18;
    --surface: #242220;
    --shadow-light: #2E2B26;
    --shadow-dark: #141310;
    --glass-fill: rgba(30, 28, 24, 0.50);
    --glass-border: rgba(255, 255, 255, 0.10);
    --accent: #E8896E;
    --accent-hover: #F0A08C;
    --success: #72B886;
    --warning: #E5A54E;
    --error: #D96B63;
    --text-primary: #EDE8DF;
    --text-secondary: #9E978C;
    --text-tertiary: #6B655D;
}
```

**Background gradient tokens** (both modes):
```css
:root {
    --bg-gradient-1: #FAF7F2;
    --bg-gradient-2: #FFF3E8;
    --bg-gradient-3: #F0F4EC;
    --bg-gradient-4: #FBF0E6;
}
[data-theme="dark"] {
    --bg-gradient-1: #1E1C18;
    --bg-gradient-2: #221F1A;
    --bg-gradient-3: #1A1D19;
    --bg-gradient-4: #201C17;
}
```

#### Theme Toggle

Implement a working dark/light toggle per the DESIGN.md strategy:

1. **`use_theme` hook** (`src/utils/theme.rs`):
   - Reads initial preference from cookie (`theme=light|dark`)
   - Falls back to `prefers-color-scheme` system preference
   - Toggles `[data-theme="dark"]` attribute on `<body>` (instant CSS variable swap, GPU-composited)
   - Persists choice to cookie for SSR consistency

2. **Toggle button** in the navbar (sun/moon icon)
3. **No flash of wrong theme** — SSR reads the cookie and sets the attribute server-side before the page renders

#### Neumorphic Utility Classes
```css
.neumo-raised { box-shadow: 6px 6px 14px var(--shadow-dark), -6px -6px 14px var(--shadow-light); }
.neumo-inset { box-shadow: inset 6px 6px 14px var(--shadow-dark), inset -6px -6px 14px var(--shadow-light); }
.neumo-card  { box-shadow: 8px 8px 20px var(--shadow-dark), -8px -8px 20px var(--shadow-light); }
```

#### Base Components (`src/components/base/`)
Simple, reusable UI primitives. Each gets its own file. These are pure presentational components — no data dependencies, no server calls.

| Component | Props | Notes |
|-----------|-------|-------|
| `Button` | `variant: ButtonVariant` (Primary, Secondary, Ghost, Danger), `disabled: bool`, `onclick` | Neumorphic-raised by default, inset on active |
| `Card` | `children` | Neumorphic card container with padding |
| `Input` | `value`, `placeholder`, `oninput` | Neumorphic-inset text input |
| `Avatar` | `src: Option<String>`, `size: AvatarSize`, `username: String` | Circular, fallback to initials |
| `EmptyState` | `icon: Element`, `title: String`, `description: String`, `action: Option<Element>` | Reusable "nothing here yet" message |

Additional shared components:

| Component | Notes |
|-----------|-------|
| `LoadingSpinner` | Simple CSS spinner for async states |
| `PageHeader` | Consistent page title + optional action button pattern |

#### Animated Background Gradient
Implement the slow-shifting background gradient (30s cycle) per DESIGN.md Layer 0. Both light and dark gradient tokens are defined; light mode is active.

### 4. Clean Up Template Boilerplate

- **Remove:** `Blog` page, `Echo` component, `Hero` component
- **Remove:** `header.svg` (not part of Noms brand)
- **Reset:** `main.css` — replace the dark-space template styles with our design tokens and utility classes
- **Update:** `index.html` title from "noms" to "Noms — Recipe Management"
- **Add:** Google Fonts import for **Nunito** (body, weights 400–700) and **Fredoka** (headings, weights 500–700) in the HTML `<head>` via `<link>` tag or `@import` in CSS
- **Update:** favicon to something appropriate (can be a simple SVG fork/chef hat icon)

### 5. Responsive Layout

Every layout component is built mobile-first from the start.

| Breakpoint | Target | Behavior |
|------------|--------|----------|
| ≥ 1024px | Desktop | Full navbar, sidebar slot visible, multi-column page layouts |
| 768–1023px | Tablet | Collapsible nav, single-column pages, touch-friendly targets |
| < 768px | Mobile | Hamburger nav, stacked single-column, bottom-sheet menus |

Specific responsive patterns:
- **Navbar:** Logo + hamburger on mobile; full links + auth on desktop. Hamburger opens a slide-out drawer with all navigation items.
- **Page layouts:** Use CSS Grid with `grid-template-columns: repeat(auto-fill, minmax(300px, 1fr))` for card grids — naturally responsive without media query hacks.
- **Touch targets:** All interactive elements minimum 44×44px (Apple HIG / WCAG guideline).
- **Viewport meta tag:** `<meta name="viewport" content="width=device-width, initial-scale=1">` in the HTML head.
- **Content padding:** Responsive `padding` on the main content area — tighter on mobile (16px), roomier on desktop (32px+).

### 6. Error Boundary

Wrap the application root in a Dioxus `ErrorBoundary` component as described in DESIGN.md § Error Handling. This catches panics in child components and renders a fallback UI instead of a blank screen.

```rust
// In App component
rsx! {
    ErrorBoundary {
        // Graceful fallback for any unhandled error
        handle: move |error| rsx! {
            div { class: "error-fallback",
                h1 { "Something went wrong" }
                p { "Please try refreshing the page." }
                button { onclick: move |_| { /* reload */ }, "Refresh" }
            }
        },
        Router::<Route> {}
    }
}
```

### 7. Dioxus Components Library Integration

Evaluate and optionally install `dioxus-components` (the official Shadcn-inspired component library) for any base components that map well. Per DESIGN.md § Component Architecture:
- Use library components where they fit (Button, Input, Card, Dialog)
- Build custom only for Noms-specific patterns (RecipeCard, ForkGraph, etc.)

If the library is stable and available via `dx add`, install it and use it for the base components. If not, build the minimal set of primitives ourselves — these are simple enough that custom implementation is fast.

## Out of Scope (explicitly not doing here)

- ❌ Any backend code — no `#[server]` functions, no database queries, no Axum routes
- ❌ OAuth or session management — auth buttons exist visually but do nothing
- ❌ Recipe CRUD or form logic — shells only
- ❌ Playwright E2E tests — visual scaffolding is hard to meaningfully test until there's interactivity. Unit tests for the `use_theme` hook and any other pure-logic utilities ARE in scope

## File Changes Summary

```
src/
├── main.rs                    # New routes, App component with ErrorBoundary
├── pages/
│   ├── mod.rs                 # Re-export all page components
│   ├── Home.rs
│   ├── Login.rs
│   ├── Dashboard.rs
│   ├── RecipeNew.rs
│   ├── RecipeDetail.rs
│   ├── CollectionList.rs
│   ├── CollectionDetail.rs
│   ├── Explore.rs
│   └── settings/
│       ├── mod.rs
│       ├── SettingsProfile.rs
│       └── SettingsAccounts.rs
├── components/
│   ├── mod.rs                 # Re-export all components
│   ├── AppLayout.rs           # Shared shell
│   ├── Navbar.rs              # Navigation bar
│   ├── Footer.rs              # Minimal footer
│   ├── base/
│   │   ├── mod.rs
│   │   ├── Button.rs
│   │   ├── Card.rs
│   │   ├── Input.rs
│   │   ├── Avatar.rs
│   │   ├── EmptyState.rs
│   │   ├── LoadingSpinner.rs
│   │   └── PageHeader.rs
│   └── ErrorFallback.rs      # Error boundary content
├── db/
│   └── mod.rs                 # Unchanged
└── utils/
    ├── mod.rs                 # Re-export theme
    └── theme.rs               # use_theme hook — dark/light toggle with cookie persistence

assets/
├── main.css                   # Replaced with design tokens + utilities
├── tailwind.css               # Unchanged (Tailwind output)
└── favicon.ico                # Updated
```

**Deleted files:**
- `assets/header.svg`
- `src/components/Hero.rs` (inline in current mod.rs)
- `src/components/Echo.rs` (inline in current mod.rs)
- `src/pages/Blog.rs` (inline in current mod.rs)
- `components/Navbar.rs` (current template version — replaced by new one)

## Acceptance Criteria

### Compilation & Dev Server
- [ ] `cargo check` passes with zero errors and zero warnings (both `wasm32` and `server` targets)
- [ ] `cargo clippy` passes with zero warnings
- [ ] `dx serve --platform server` starts without errors

### Routes & Navigation
- [ ] Navigating to `/` shows the landing page with correct branding
- [ ] Navigating to `/login` shows sign-in buttons (not wired)
- [ ] Navigating to `/dashboard` shows empty state
- [ ] Navigating to `/recipes/new` shows empty form shell
- [ ] Navigating to `/recipes/1` shows recipe detail placeholder
- [ ] Navigating to `/collections` and `/collections/1` shows collection pages
- [ ] Navigating to `/explore` shows explore placeholder
- [ ] Navigating to `/settings/profile` and `/settings/accounts` shows settings pages
- [ ] Navbar renders on every page with logo, nav links, auth slot, and theme toggle

### Dark Mode
- [ ] `[data-theme="dark"]` attribute on `<body>` triggers dark palette (all CSS tokens swap)
- [ ] Theme toggle button in navbar visually indicates current mode (sun ☀️ / moon 🌙)
- [ ] Clicking theme toggle switches between light and dark modes
- [ ] Preference persists across page reload (cookie-based)
- [ ] Initial render respects `prefers-color-scheme` system preference (no cookie = follow system)
- [ ] All base components (Button, Card, Input, etc.) look correct in both modes
- [ ] Background gradient tokens swap correctly between light and dark palettes

### Responsive Layout
- [ ] Navbar collapses to hamburger menu at < 768px viewport width
- [ ] Hamburger menu opens a slide-out navigation drawer
- [ ] All interactive elements are minimum 44×44px touch targets
- [ ] Page layouts stack to single column at < 768px
- [ ] Content area padding is tighter on mobile (16px) than desktop (32px+)
- [ ] No content overflow, horizontal scrollbars, or broken layouts at 320px–1920px widths
- [ ] `meta viewport` tag is present in the HTML head

### Design System
- [ ] All 15 light mode CSS variables defined on `:root` with correct warm earth-tone values
- [ ] All 15 dark mode CSS variables defined under `[data-theme="dark"]` with correct values
- [ ] Background gradient tokens defined for both modes
- [ ] Neumorphic shadow classes (`.neumo-raised`, `.neumo-inset`, `.neumo-card`) produce visible raised/inset effects
- [ ] Background gradient animates slowly (30s cycle)
- [ ] Navbar has glassmorphic backdrop blur

### Components
- [ ] `Button` renders all variants (Primary, Secondary, Ghost, Danger) with correct styling
- [ ] `Card` renders as a neumorphic container
- [ ] `Input` renders with neumorphic-inset styling
- [ ] `Avatar` renders image when `src` is provided, falls back to initials otherwise
- [ ] `EmptyState` renders icon, title, description, and optional action
- [ ] `LoadingSpinner` animates visually
- [ ] `PageHeader` renders title with optional action slot
- [ ] Error boundary renders fallback UI when a component panics

### Cleanup
- [ ] Template boilerplate (Blog page, Echo, Hero, header.svg) is fully removed
- [ ] `main.css` no longer contains dark-space-template styles
- [ ] `cargo test` passes (theme utility has unit tests)

## Outcome

A visually coherent application shell that looks and feels like Noms, in both light and dark modes, on any screen size from phone to desktop. All routes render appropriate placeholder pages. The design system (CSS variables, neumorphic utilities, base components, theme hook) is established and ready for every subsequent feature issue to use. No backend code was touched — this is pure frontend scaffolding that the rest of Phase 1 fills in.
