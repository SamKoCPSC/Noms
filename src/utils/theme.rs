use dioxus::prelude::*;

/// Reactive theme state returned by [`use_theme`].
#[derive(Clone, Copy, PartialEq)]
pub struct UseTheme {
    dark: Signal<bool>,
    toggle: Callback<()>,
}

impl UseTheme {
    /// Returns `true` if dark mode is active.
    pub fn is_dark(&self) -> bool {
        (self.dark)()
    }

    /// Toggles between light and dark mode.
    pub fn toggle(&self) {
        (self.toggle)(())
    }
}

/// Reads the saved theme from `localStorage` (WASM-only).
/// Returns `false` on server/desktop.
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

/// Hook for managing light/dark theme with localStorage persistence.
///
/// - Hydrates with the server's light state, then restores `localStorage.theme`
///   on WASM so labels and switch semantics update with the page colors.
/// - Whenever the value changes, a [`use_effect`] syncs the `dark` class on
///   `<html>` and writes the preference back to `localStorage`.
///
/// Returns a [`UseTheme`] handle (Copy) that can be passed as a component prop.
pub fn use_theme() -> UseTheme {
    // Match SSR during hydration, then restore the saved preference.
    let mut is_dark = use_signal(|| false);
    #[cfg(target_arch = "wasm32")]
    let mut restored = use_signal(|| false);
    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        is_dark.set(read_saved_theme());
        restored.set(true);
    });

    // ── side‑effect: keep <html> class + localStorage in sync ───────
    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        if !restored() {
            return;
        }
        let dark = is_dark();
        spawn(async move {
            let script = if dark {
                "document.documentElement.classList.add('dark'); \
                 localStorage.setItem('theme', 'dark')"
            } else {
                "document.documentElement.classList.remove('dark'); \
                 localStorage.setItem('theme', 'light')"
            };
            let _ = document::eval(script).await;
        });
    });

    // ── toggle callback ──────────────────────────────────────────────
    let toggle = use_callback(move |()| {
        is_dark.toggle();
    });

    UseTheme {
        dark: is_dark,
        toggle,
    }
}
