# Dioxus Agent Guide

Dioxus is a cross-platform UI framework for Rust, similar to React. It compiles to web (WASM), desktop (webview), mobile (iOS/Android), and native (GPU-rendered). The official Dioxus documentation can be found at https://dioxuslabs.com/learn/0.7/

## Quick Overview

- **Language**: Rust (stable toolchain)
- **UI Model**: React-like with VirtualDOM, components, hooks, signals
- **Syntax**: JSX-like `rsx!` macro for declaring UI
- **Platforms**: Web, Desktop (Windows/macOS/Linux), Mobile, Native, LiveView (server-rendered)

## Architecture Documentation

For deeper understanding, see `DioxusDocs`:

| When working on...                         | Read...            |
| ------------------------------------------ | ------------------ |
| VirtualDOM, components, diffing, events    | `01-CORE.md`       |
| CLI, build system, bundling, dev server    | `02-CLI.md`        |
| RSX macro, parsing, formatting             | `03-RSX.md`        |
| Signals, state management, reactivity      | `04-SIGNALS.md`    |
| Server functions, SSR, hydration           | `05-FULLSTACK.md`  |
| Web/desktop/native/liveview renderers      | `06-RENDERERS.md`  |
| Hot-reload, hot-patching, devtools         | `07-HOTRELOAD.md`  |
| Asset macro, manganis, const serialization | `08-ASSETS.md`     |
| Router, navigation, nested routes          | `09-ROUTER.md`     |
| WASM code splitting                        | `10-WASM-SPLIT.md` |

## Key Concepts

- **VirtualDOM**: Tree of `VNode` with templates, dynamic nodes, and attributes
- **Signals**: Copy-able reactive primitives via generational-box (generation-based validity)
- **WriteMutations**: Trait that renderers implement to apply DOM changes
- **RSX**: Proc macro that compiles JSX-like syntax to `VNode` construction
- **Server Functions**: `#[server]` macro generates client RPC stubs and server handlers
- **Subsecond**: Hot-patches Rust code via jump table indirection (no memory modification)
- **Manganis**: `asset!("/main.css")` macro for including assets by embedding data via linker symbols

## Common Patterns

**Component definition**:
```rust
#[component]
fn MyComponent(name: String) -> Element {
    let mut count = use_signal(|| 0);
    rsx! {
        button { onclick: move |_| count += 1, "{name}: {count}" }
    }
}
```