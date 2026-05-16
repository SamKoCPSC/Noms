# Dark Mode Implementation — Lessons Learned

**Date:** 2026-05-16
**Issue:** NOMS-003 (UI Scaffold & Application Shell)
**Status:** Resolved

---

## Summary

Implementing a working dark mode toggle with localStorage persistence in Dioxus 0.7
fullstack turned out to be far more involved than expected. This document records every
pitfall encountered, the root cause, and the final solution so future developers don't
repeat the same mistakes.

---

## Problem 1: Independent signals per component

### Symptom
Clicking the theme toggle in the navbar changed the button icon (moon → sun), but the
page background did **not** switch to dark mode.

### Root Cause
Both `AppLayout` and `Navbar` called `use_theme()`, which internally called
`use_signal(|| false)`. In Dioxus, `use_signal` creates a **new** signal every time the
hook is called. So `AppLayout` had Signal A and `Navbar` had Signal B — two completely
independent pieces of state. Toggling in the navbar mutated Signal B, but `AppLayout`
was reading Signal A.

```
AppLayout ── use_theme() ──▶ Signal A (always false)
Navbar    ── use_theme() ──▶ Signal B (toggled, but nobody reads it)
```

### Attempted Fix: External `theme` crate
We evaluated the [`theme`](https://crates.io/crates/theme) crate (v0.0.3), which provides
a `ThemeProvider` with context-based state sharing. **Rejected** because:
- Depends on Dioxus **0.6.3** — incompatible with our **0.7.1** project
- Would pull two Dioxus versions simultaneously (compile failure)
- Only 3 GitHub stars, 12% documented, unclear maintenance
- `ColorTokens` has only 7 fields — far too limited for our 30+ CSS custom properties

### Attempted Fix: `GlobalSignal`
We replaced `use_signal` with `Signal::global(|| false)` (Dioxus 0.7's built-in global
state). This worked technically but was reverted because the team preferred a
prop-based approach for explicit data flow.

### Final Solution: Lift state + pass as prop
`AppLayout` owns the signal (calls `use_theme()` once) and passes the resulting
`UseTheme` handle down to `Navbar` as a prop.

```rust
// AppLayout — the single source of truth
let theme = use_theme();
rsx! {
    Navbar { theme }
}

// Navbar — receives the same signal via props
pub fn Navbar(theme: UseTheme) -> Element { ... }
```

`UseTheme` needed `PartialEq` added to its derive macro so it could be used as a
Dioxus component prop:

```rust
#[derive(Clone, Copy, PartialEq)]  // ← PartialEq was missing
pub struct UseTheme { ... }
```

**Key insight:** `Signal<T>` and `Callback<T>` both implement `PartialEq` in Dioxus 0.7,
so adding the derive was all that was needed.

---

## Problem 2: `document::eval` crashes the app

### Symptom
After adding a `use_effect` that called `document::eval(...)` to sync the `<html>` class,
the entire app crashed on load with the error boundary ("Something went wrong"). No
console errors were visible.

### Root Cause
In Dioxus 0.7, `document::eval` returns an `Eval` object that **must be awaited**.
Calling it without `.await` inside `use_effect` causes a panic at runtime.

```rust
// ❌ WRONG — crashes the app
use_effect(move || {
    let _ = document::eval("document.documentElement.classList.add('dark')");
});

// ✅ CORRECT — must be awaited
use_effect(move || {
    spawn(async move {
        let _ = document::eval("document.documentElement.classList.add('dark')").await;
    });
});
```

**Why no console error?** The panic happens inside the WASM runtime before it reaches
the browser's error handler. The Dioxus `ErrorBoundary` catches it and renders the
fallback, but the underlying error is swallowed.

### How to debug this
1. Remove the suspected code incrementally until the app loads
2. Re-add pieces one at a time to isolate the crash
3. Use Chrome DevTools to check if the app renders at all (snapshot) vs. hits error
   boundary

---

## Problem 3: `web-sys::window()` panics during SSR

### Symptom
After adding `web_sys::window()` to read from `localStorage` in the signal initializer,
the app crashed again on load — even though the code had `if let Some(window) = ...`
which should have been safe.

### Root Cause
Dioxus fullstack renders on the **server first**, then hydrates on the client. During
server rendering, `web_sys::window()` doesn't just return `None` — it **panics** because
the `web-sys` crate is compiled for the `wasm32-unknown-unknown` target but the server
binary runs on `x86_64`.

The `if let Some(...)` pattern only handles the `None` case at runtime; it doesn't
prevent the code from being **compiled and linked** into the server binary where the
`web-sys` symbols don't exist.

### Final Solution: `#[cfg(target_arch = "wasm32")]` gating

```rust
#[cfg(target_arch = "wasm32")]
fn read_saved_theme() -> bool {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(theme)) = storage.get_item("theme") {
                return theme == "dark";
            }
        }
    }
    false
}

pub fn use_theme() -> UseTheme {
    let mut is_dark = use_signal(|| {
        #[cfg(target_arch = "wasm32")]
        return read_saved_theme();

        #[cfg(not(target_arch = "wasm32"))]
        false
    });
    // ...
}
```

The `#[cfg(...)]` attribute is a **compile-time** gate. Code behind
`#[cfg(target_arch = "wasm32")]` is never compiled into the server binary, so it can't
panic there.

---

## Problem 4: `use_effect` doesn't reliably run during client hydration

### Symptom (earlier, during initial scaffold)
When using `use_effect` with `#[cfg(target_arch = "wasm32")]` to detect the system
theme preference, the effect would run during server rendering but not re-fire on the
client after hydration.

### Root Cause
`#[cfg(target_arch = "wasm32")]` is a compile-time check. If the effect was compiled
into the WASM binary but the effect's closure captured server-rendered state, the
client might not re-execute it during hydration.

### Resolution
This was resolved by moving the DOM sync logic into `document::eval` inside a
`spawn(async move { ... })` block, which always executes on the client after the
component mounts.

---

## Architecture: How it all fits together

```
┌─────────────────────────────────────────────────────────────┐
│                        AppLayout                             │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  use_theme()  ◄── single call, owns the signal       │    │
│  │                                                       │    │
│  │  Signal<bool> (is_dark)                               │    │
│  │    │                                                  │    │
│  │    ├─▶ passed as prop ──▶ Navbar { theme }            │    │
│  │    │                    Navbar reads + toggles        │    │
│  │    │                                                  │    │
│  │    └─▶ use_effect ──▶ spawn(async {                   │    │
│  │                      document::eval(...).await        │    │
│  │                   })                                  │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                             │
│  Signal initializer:                                        │
│    #[cfg(wasm32)] → reads localStorage                      │
│    #[cfg(not wasm32)] → defaults to false                   │
└─────────────────────────────────────────────────────────────┘
```

### Data flow on toggle

1. User clicks toggle button in `Navbar`
2. `theme.toggle()` flips `is_dark` signal (`false` → `true`)
3. `use_effect` detects the signal changed (reactive subscription)
4. `spawn(async move { ... })` runs on the client
5. `document::eval(...).await` executes JavaScript:
   - Adds `dark` class to `<html>`
   - Writes `"dark"` to `localStorage`
6. CSS custom properties cascade from `<html>` to all elements

### Data flow on page load

1. Server renders HTML (light mode, no `dark` class)
2. WASM loads on client
3. `use_signal` initializer runs:
   - `#[cfg(wasm32)]` → reads `localStorage.getItem("theme")`
   - Returns `true` if `"dark"`, `false` otherwise
4. `use_effect` fires with the restored value
5. `document::eval` syncs `<html>` class to match the restored state

---

## Cargo.toml changes

```toml
[dependencies]
web-sys = { version = "0.3.98", features = ["Window", "Storage"] }
```

`web-sys` is already a transitive dependency of `dioxus-web`, but we need to
explicitly add it with the `Window` and `Storage` features enabled.

---

## Files modified

| File | Change |
|------|--------|
| `src/utils/theme.rs` | localStorage init, `use_effect` sync, `#[cfg]` gating |
| `src/utils/theme.rs` | Added `PartialEq` to `UseTheme` derive |
| `src/components/navbar.rs` | Accepts `theme: UseTheme` as prop instead of calling `use_theme()` |
| `src/components/app_layout.rs` | Drops conditional class; `use_effect` manages `<html>` |
| `Cargo.toml` | Added `web-sys` dependency with `Window`, `Storage` features |

---

## Quick reference: Dioxus 0.7 `document::eval` rules

| Rule | Detail |
|------|--------|
| **Must await** | `document::eval(...)` returns `Eval` — call `.await` on it |
| **Inside effects** | Use `spawn(async move { ... })` inside `use_effect` |
| **Inside events** | Use `move \|_\| async move { ... }` in event handlers |
| **Never in component body** | Runs before DOM is mounted — will fail silently |
| **No automatic return** | Use `return "value"` in JS, or `dioxus.send(value)` for async |
