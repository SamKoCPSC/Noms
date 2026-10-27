# Awkward ingredient wording review (2026-10-27)

The user approved changing `berry (impregnated with sugar)` to **sweetened berries**.
UUID `cf96958f-b8e3-4fb8-9450-d7f82d86f183`, description, status, provenance and
source mapping stay unchanged. Both `berry (impregnated with sugar)` and
`berries (impregnated with sugar)` remain searchable aliases.

The pinned [FoodOn entry FOODON_03316906](https://purl.obolibrary.org/obo/FOODON_03316906)
has SIREN annotations for whole berries, not heat-treated, with sugar or sugar
syrup added. The name does not imply drying, candying or a particular maceration
technique. The berry amendment moved that ID from hold to change. The user then
approved the 12 wording replacements below, all now applied in the current PR's
readable-label ledger: **488 renames, 1,009 holds, 139 alias promotions and 349 net
new aliases**. Current totals are **13,532 active, 85 retired and 10,094 aliases**.
Five approved IDs extend the original 1,492-ID scope to 1,497 reviewed IDs; seven
move from hold to change. No ingredient IDs, definitions or source mappings change.
Earlier curation-stage pins and the original 475-rename download stay frozen.

## Follow-up review

A fresh lexical screen inspects all **13,617 preferred labels**, including all
**13,532 active ingredients**. Patterns flag **564 active labels** for obsolete or
industrial wording, packaging/use language, classification/anatomical qualifiers,
catalogue punctuation, wordy relations or dense names. These flags include valid
labels and are not a count of errors. Manual review yields the **36 candidates**
below, with proposed names checked against every active preferred label and alias.
This is a targeted screen, not a guarantee of finding every awkward expression.

The berry label and the 12 approved wording replacements are now changed.
The other 24 candidates remain unchanged; none of these decisions merges IDs.

### Twelve applied wording changes

| Prior label | New primary label | Meaning preserved |
| --- | --- | --- |
| beverage powder for home preparation of drinks | powdered drink mix | Dry base, drink type unspecified |
| drink mix for the preparation of alcoholic beverage | alcoholic drink mix | A mix for preparing alcoholic drinks; may not itself contain alcohol |
| beverage syrup cola type | cola syrup | Syrup, not ready-to-drink cola |
| savoury sauce dry preparation | dry savory sauce mix | Dry mix, not prepared sauce |
| candy with ground nutmeats | candy with ground nuts | Ground nuts and candy formulation |
| spring roll pastry case | spring roll wrappers | Spring-roll identity; flour type unspecified |
| wonton pastry (case) | wonton wrappers | Source explicitly gives `wonton wrapper` as an exact synonym |
| vegetable juice (gassed) | carbonated vegetable juice | Source explicitly specifies carbonation |
| fish roe (salt treated) | salted fish roe | Salt added; distinct from sugar-treated roe |
| fish roe (sugar treated) | sweetened fish roe | Sugar or syrup added; not necessarily dried or candied |
| english walnut meat | shelled English walnuts | Definition specifies walnut pieces without shell |
| water in sealed container | packaged water | Container unspecified; not necessarily bottled |

### Sixteen candidates needing definition checks

| Current label | Review lead or unresolved meaning |
| --- | --- |
| shiu my pastry case | Likely shumai wrappers; confirm transliteration and wrapper identity |
| liver based spreadable-textured specialities | Consider liver spread, but preserve the broad spread/textured scope |
| confection (masticatory) | Consider chewy confectionery; confirm whether gum is included |
| confection (quiescently frozen) | Confectionery frozen without stirring; retain the freezing-process distinction |
| dextrose containing crystallization water | Hydrated dextrose; monohydrate is plausible but hydration stoichiometry is not established by the pinned annotation |
| butter (renovated) | Recognized processing category; establish the correct plain-language equivalent |
| cream (reactivated) | Do not infer reconstituted cream from the sparse annotation |
| honey (decharacterized) | Do not infer filtered or deodorized honey |
| beef steak (fabricated) | Fabricated may refer to butchery or product formation; not automatically restructured steak |
| veal steak (fabricated) | Same ambiguity, while retaining veal identity |
| chicken thigh retorted in gravy | Preserve heat processing, gravy and source bone/skin qualifiers; packaging is not established |
| retorted chicken thigh | Establish a consumer-friendly equivalent without dropping heat treatment |
| sweet potato (skin possible) | Skin status unspecified, not necessarily skin-on |
| red variety sweet potato (skin possible) | Retain red variety and unspecified skin status |
| starch syrup (concentrated solid form) | Consider starch syrup solids; do not assume corn origin or maltodextrin |
| nut meat | Source specifies seed skin removed, germ present; shelled nuts may lose that skin qualifier |

### Four candidates with name-ownership conflicts

| Current label | Natural wording already owned by another active ID |
| --- | --- |
| meat based spreadable-textured specialities | meat spread |
| canned-tinned meat | canned meat |
| chicken thigh (debonned) | boneless chicken thigh |
| chicken thigh (skinless, debonned) | boneless skinless chicken thigh |

These require identity/definition review rather than renaming onto an existing
owner. Their existing names remain available.

### Four scope candidates

| Current label | Issue |
| --- | --- |
| brewers grains (exhausted) | Spent brewing grains is more familiar, but this source entry also specifies cattle as consumer |
| lettuce head (anatomical) | A plant-structure entry; review the relationship to culinary lettuce identities |
| U.S. code of federal regulations, title 21 type | A classification reference rather than a cooking ingredient |
| corn for degermination | A use-purpose/processing category; review suitability for recipe selection |

Ordinary technical names such as desiccated coconut, cocoa mass, refined olive
oil, soy protein concentrate, reconstituted milk and transglutaminase preparation
remain useful. A technical word alone is not grounds for changing ingredient scope.

## Evidence and verification

The amended seed checksum is
`3de52f891231718606912b69dd80eaf4dd304d86ac5ea9b9eb2051599618b483`.
The four-source checksum remains
`1cb1ba876c7623c323889671bf87396e592fc1e2517b2a24a62fb8be623f181c`.
The existing label builder replays the amended ledger from the unchanged
`ec68a2b3d00c3cfe7ba518d73e1f09766375d930` input and restores that baseline exactly.
All 135 vocabulary Python tests pass, including canonical validation, collision
checks, historical-stage restoration and preservation of every exact old name.
The old and new names of all 12 approved entries and the berry resolve to their
original UUIDs in the canonical data. All 24 pending entries remain unchanged.
These amendments have not been seeded into the local or production database.

External artifacts at `/workspace/vocabulary-awkward-language-review` contain the
reproducible read-only screen, every label, the 564 flagged labels, all 36 curated
candidates with UUIDs/conflicting owners/definitions, the pinned upstream source
annotations, the berry-stage seed and verification log. Those discovery artifacts
remain frozen. The current 12-wording update, before/after seed, approved decisions
and verification log are at `/workspace/vocabulary-awkward-wording-update`.
Raw audits remain outside
the repository; only this compact review, the amended ledger/seed and related
tests/documentation are maintained here.
