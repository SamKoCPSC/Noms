# Precise ingredient unit suggestions

Recipe creation/editing, pantry creation/editing and grocery creation/editing
share an optional client-side combobox. Its 24 suggestions are bundled in
`src/units.rs`; no vocabulary table, network lookup, manifest or cache is needed.

| Family | Suggested measures |
| --- | --- |
| Weight | mg, g, kg, oz, lb |
| Metric volume | mL, cL, dL, L |
| US volume | US tsp, US tbsp, US cup, US fl oz, US pt, US qt, US gal |
| Metric kitchen measures | metric tsp (5 mL), metric tbsp (15 mL), metric cup (250 mL) |
| Australian measure | AU tbsp (20 mL) |
| Imperial volume | Imperial fl oz, Imperial pt, Imperial qt, Imperial gal |

Parenthetical sizes above explain the measures; the saved labels are `metric
tsp`, `metric tbsp`, `metric cup` and `AU tbsp`. US cups mean customary cups;
US pints, quarts and gallons are liquid measures. Generic queries such as `cups`
and `tablespoons` offer the explicitly qualified choices without choosing a
regional standard automatically. Weight ounces and fluid ounces remain distinct.

Aliases include common abbreviations, full names, plurals, US/UK spellings and
punctuation variants: `grammes`, `milliliters`, `lbs.` and `fl. oz.`. At most eight
results appear, starting with exact matches and then prefixes. Multiword queries
can reverse the qualifier order, for example `cups metric`.

The field stays free text. Only an explicit mouse/touch selection or active
keyboard selection writes a preferred label. Loading, typing, tabbing away or
saving does not rewrite a unit. Blank units and unusual custom measures still
work. Approximate measures, counts and containers such as pinch, handful, clove
or can are not suggested. Quantities, conversion rules, existing recipe data
and grocery/pantry matching semantics are unchanged.

The control supports arrows, Enter, Escape and Tab, retains form length/disabled
constraints, exposes combobox/listbox semantics and uses the ingredient menu
styles. Escape closes the suggestion list before an enclosing edit dialog.

## Verification

```sh
cargo test --locked --no-default-features --features server --lib units::tests
python3 scripts/tests/test-unit-autocomplete-browser.py \
  --instance noms-example --output /tmp/noms-unit-browser
```

The browser test needs a ready, isolated local mock-auth environment, Chromium,
Playwright and Docker, and creates synthetic private fixtures. `--base-url` can
provide an explicit loopback URL if runtime metadata is unavailable. It checks
recipe/draft persistence, published-recipe edit controls, pantry/grocery unit
round-trips, keyboard/mouse/touch, custom/blank values, regional ambiguity,
distinct ounces, absence of unit API calls, and dropdown geometry at four widths.
