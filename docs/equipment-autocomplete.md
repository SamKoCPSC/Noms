# Recipe equipment autocomplete

Recipe creation, draft editing and published-recipe editing share the same
optional equipment combobox. After two characters, it searches locally and
shows up to eight preferred names. Labels, aliases, word prefixes and reviewed
broad search terms are matched with NFKC/case/whitespace normalization. Exact
preferred names rank first, then exact alternative names or broad search
terms, then prefixes; ties sort alphabetically.

Click or tap a suggestion to insert its preferred label. Arrow keys highlight
options and Enter chooses the highlighted option. Escape, Tab and blur close
the menu without changing the entered text. No option is selected merely by
opening or closing the list. Custom equipment names and existing recipe text
remain valid, including names absent from the catalog. Ordinary draft saving,
publication, import/export and recipe history continue to store the equipment
name as text; suggestions do not introduce a new required identifier.

## Maintained catalog

`data/vocabulary/equipment.json` is the runtime catalog promoted from the
reviewed 2026-10-28 equipment draft. It contains 427 equipment names, 16 related
supplies and 316 aliases. Supplies such as parchment paper are included because
they are useful recipe requirements. Each record retains its equipment/supply
scope. The source research snapshot remains historical evidence; future runtime
edits belong in this maintained catalog.

Broad terms such as `mixer`, `kettle`, `strainer` and `sandwich maker` explicitly
target several entries. These search terms are not aliases for one subtype and
do not assert substitution or compatibility. Sugarcraft gun and lollipop mold
remain unresolved and are not assigned to another tool.

The roughly 32 KB of compact JSON is embedded in the app and normalized once
per app runtime. Keystrokes perform no API or database requests. There is no
equipment database seed or separately persisted equipment cache. Normal app
asset versioning delivers updated vocabulary on a fresh page load after a
deployment; an already open page continues using its loaded app version until
reloaded. Ingredient vocabulary downloads and their cache remain independent.

## Dropdown layout

Equipment, ingredient and unit suggestions share an adaptive anchored popover.
It prefers the space below the input, flips above when there is more usable
space there, and limits its width and scrolling height to the visual viewport.
Keyboard resizing, viewport panning and page/container scrolling reposition the
surface. Without VisualViewport support, it uses the layout viewport instead.
The native nonmodal top layer avoids clipping in scrollable forms and dialogs;
focus remains in the input. Mobile bottom-sheet dialogs also fit the visual
viewport so their focused fields remain reachable above the keyboard.

Rows have no inter-item margins, compact padding, 28-pixel minimum desktop
targets and 32-pixel minimum touch targets. Arrow navigation scrolls only the
suggestions list; manually scrolling it does not snap back to an earlier
highlight. Closing or unmounting a list removes its viewport listeners and
cancels pending layout frames. Matching and confirmation behavior are unchanged.

## Verification

```sh
cargo test --locked --lib equipment::
cargo check --locked --target wasm32-unknown-unknown
cargo check --locked --no-default-features --features server
python3 scripts/tests/test-equipment-autocomplete-browser.py \
  --instance noms-cloud --base-url http://127.0.0.1:PORT
python3 scripts/tests/test-autocomplete-viewport-browser.py \
  --base-url http://127.0.0.1:PORT
```

The browser check requires the local mock-auth development stack, Chromium and
Python Playwright. It creates private test data, checks creation/draft/existing
recipe flows, mouse/keyboard/touch selection, distinct broad-term suggestions,
free-text preservation, row removal, saved values and local/offline matching.
The viewport test simulates keyboard shrinkage and panning through browser
VisualViewport measurements/events, and checks real rendered bounds, spacing,
focus, touch selection, scroll behavior, native dialogs and the layout-viewport
fallback. Headless Chromium does not display a physical OS keyboard.

On mobile, focused autocomplete fields can also scroll toward the visible top when the keyboard opens and there is insufficient space. The page or sheet receives temporary scroll room for fields near its end. This helper handles only field scrolling; the shared popup continues to own result positioning, clipping avoidance, compact row sizing, and list-only keyboard navigation. See `scripts/tests/test-autocomplete-keyboard-browser.py` for normal-flow field scrolling and keyboard transitions.
