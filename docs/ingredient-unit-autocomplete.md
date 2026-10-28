# Precise ingredient unit suggestions

Recipe creation/editing, pantry creation/editing and grocery creation/editing
share a required client-side unit combobox. Its 24 suggestions are bundled in
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
and `tablespoons` offer the explicitly qualified choices. Confirming an
unqualified query uses the first result (the US measure); an explicitly selected
metric, Australian or Imperial option keeps that standard. Weight ounces and
fluid ounces remain distinct.

Aliases include common abbreviations, full names, plurals, US/UK spellings and
punctuation variants: `grammes`, `milliliters`, `lbs.` and `fl. oz.`. At most eight
results appear, starting with exact matches and then prefixes. Multiword queries
can reverse the qualifier order, for example `cups metric`.

The menu uses single-line options in a scrollable panel capped at 10rem.
Enter, Tab, Escape and blur confirm a nonblank query's active result, defaulting
to its first result. Explicit pointer selections win. Blank inputs and unmatched
text require correction. An empty field is never silently assigned grams;
authors must select a measure and enter the corresponding amount.
Partially typed queries stay local until confirmation and cannot leak into
autosave. Invalid committed units pause recipe autosave and fail client form
validation, so they cannot leave draft creation in an unconfirmed state.

New authoring requests accept supported units only. Full names, plural names,
punctuation variants and complete aliases normalize to the same saved labels as
the UI default. The API rejects partial queries, unknown strings, counts,
containers and approximate measures such as `pinch`, `can` or `packet`.
Blank and whitespace-only units are invalid. Whole portions such as eggs need
a measured quantity, for example `100 g` rather than `2` with no unit; the app
does not guess a conversion. Validation occurs before database writes for recipe
create/update, draft create/save/publication, JSON import and manual pantry/
grocery creation and field edits. Multi-item validation failures write nothing.

No schema or stored-data migration is involved. Historical records, exports,
version restoration, pantry copies and recipe-generated grocery provenance keep
their original data. Saving edited fields with an unsupported legacy unit
requires choosing a supported unit, including for legacy blank units. Existing
draft publication requires a prior save with standardized units. Amounts and
numeric conversion rules are unchanged.

The control supports arrows, Enter, Escape and Tab, retains form length/disabled
constraints, exposes combobox/listbox semantics and uses the ingredient menu
styles. Keyboard navigation scrolls the active option into view. Escape confirms
and closes the suggestion list before an enclosing edit dialog. Touch targets
remain at least 44px; desktop rows use 32px.

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
round-trips, keyboard/mouse/touch, default confirmation, blank values and invalid
units, regional ambiguity, distinct ounces, absence of unit API calls, compact
dropdown geometry at four widths, and direct authenticated API bypass attempts.
Rejected API requests are checked against hashes of the stored recipe, draft,
version, pantry and grocery rows to prove that no partial write occurred.
