# Kitchen suitability review (2026-10-27)

The user directed a final disposition for all 24 remaining entries from the
[awkward-language review](AWKWARD_LABELS.md): retain recognizable cooking
ingredients under natural names and remove unsuitable or redundant selections.
This review renames **3** entries and retires **21** from active suggestions.
Every candidate has a completed decision; none is awaiting wording approval.

Retirement preserves the original UUID, label, aliases, definition and source
mappings with `status: retired`. Existing recipe references remain valid. The
server lookup and public client catalogue select active entries only. No old
recipe names or ingredient IDs are reassigned to another ingredient, and no
retired names are transferred to unrelated active entries.

## Three retained ingredients

| Previous label | Preferred label | Rationale |
| --- | --- | --- |
| shiu my pastry case | shumai wrappers | User explicitly retained the recognizable dumpling wrapper; also searchable as shumai wrapper and siu mai wrapper(s) |
| liver based spreadable-textured specialities | liver spread | A familiar prepared cooking ingredient, with liver species/formula still unspecified |
| nut meat | peeled nuts | The pinned source specifies seed skin removed, germ present; retain that distinction from shelled nuts |

These retain their existing UUIDs, source mappings and definitions. Old labels
remain aliases. Shumai, wonton and spring-roll wrappers remain separate, as do
peeled and shelled nuts. No new ingredients are added.

## Eight entries removed in favor of existing kitchen selections

| Retired label | Existing selection(s) retained |
| --- | --- |
| meat based spreadable-textured specialities | meat spread |
| canned-tinned meat | canned meat |
| chicken thigh (debonned) | boneless chicken thigh |
| chicken thigh (skinless, debonned) | boneless skinless chicken thigh |
| sweet potato (skin possible) | sweet potato |
| red variety sweet potato (skin possible) | red variety sweet potato |
| lettuce head (anatomical) | lettuce head |
| dextrose containing crystallization water | dextrose, dextrose monohydrate, dextrose anhydrous |

These alternatives preserve useful ingredient coverage without retaining the
overtechnical selections. The list is navigation guidance, not an assertion of
exact chemical/formulation equivalence or permission to reassign saved recipes.
For example, unspecified hydrated dextrose is not silently made a monohydrate.

## Thirteen unsuitable selections removed

| Retired label | Reason |
| --- | --- |
| brewers grains (exhausted) | Source explicitly specifies cattle feed; not a food-grade brewing-grain ingredient |
| corn for degermination | Industrial processing/use-purpose category |
| U.S. code of federal regulations, title 21 type | Regulatory classification, not food |
| confection (masticatory) | Ambiguous confectionery class; recognizable chewy candy and chewing gum remain available |
| confection (quiescently frozen) | Manufacturing-process class, not a specific ordinary cooking ingredient |
| butter (renovated) | Specialist reprocessing category |
| cream (reactivated) | Unclear specialist processing category |
| honey (decharacterized) | Unclear industrial modification category |
| beef steak (fabricated) | Ambiguous processing/formation category |
| veal steak (fabricated) | Ambiguous processing/formation category |
| chicken thigh retorted in gravy | Industrial heat-processing-specific product |
| retorted chicken thigh | Industrial heat-processing-specific product |
| starch syrup (concentrated solid form) | Unspecified industrial syrup-solids category |

The review is limited to the 24 identified candidates. It does not remove
specialty culinary ingredients simply because they are uncommon, or conflate
processing/formulation differences that affect cooking. In particular, no
retorted product is silently converted to raw chicken and no unspecified syrup
solid is relabeled as corn syrup or maltodextrin.

## Reproduction and verification

The separate [ledger](../../data/vocabulary/adapters/ingredient-kitchen-suitability-review.json)
follows commit `3b2af85bf9294d4ab12fab13f4736d210e481d25` and uses the existing
gap-review builder's reviewed renames and explicit retirements. Earlier ledgers
and checksums remain frozen. `maintained_readable_stage` reverses this layer
before the readable-label and historical ingredient-stage tests.

- Input seed SHA-256: `3de52f891231718606912b69dd80eaf4dd304d86ac5ea9b9eb2051599618b483`
- Output seed SHA-256: `e1239f102a74622579d5c7ac99c0499930cc77c9b147991eec8574235409feb5`
- Unchanged sources SHA-256: `1cb1ba876c7623c323889671bf87396e592fc1e2517b2a24a62fb8be623f181c`
- Current totals: **13,511 active, 106 retired, 13,617 total; 10,088 active aliases**.

With the input seed and sources from that commit in a fresh external directory:

```sh
python3 scripts/vocabulary/fill_gaps.py \
  --seed /tmp/kitchen-base/ingredients.json \
  --sources /tmp/kitchen-base/sources.json \
  --policy data/vocabulary/adapters/ingredient-kitchen-suitability-review.json \
  --output /tmp/kitchen-replay
python3 -m unittest discover -s scripts/vocabulary -p 'test_*.py'
```

All **140 vocabulary Python tests** pass, including exact forward/reverse replay,
the complete 24-entry scope, identity preservation, old-name retention for the
three renames, inactive-name exclusion and unchanged earlier curation stages.
The Rust validator accepts all 13,617 records and four sources without warnings.
The local dry run rolls back with no database changes. Loading into the running
mock-auth deployment applies 37 updates: this review's 24 plus the previously
unloaded berry and 12-wording amendments. It adds no IDs, retires 21 and updates
no sources. A repeated `--if-changed` load skips with no writes. Recipe/draft/
version and source/mapping hashes remain unchanged.

The authenticated live catalogue contains exactly 13,511 active IDs and excludes
all 21 retired IDs. Chromium verifies all 11 new/legacy names of the three
retained ingredients, and confirms that the 21 retired labels are not offered.
There are no per-query server calls or page errors. Recipe/draft/version hashes
also remain unchanged after browser checks. Verification artifacts remain
external at `/workspace/vocabulary-kitchen-review`. No production deployment occurs.
