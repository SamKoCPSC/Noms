# FoodOn candidate label review

Reviewed snapshot: 2026-09-20 (cd73540243a84bcd511a500d9a497d12d7dc02f6).

The extracted catalog has **29,239 classes**, of which **20,152** pass the adapter's structural scope checks. Structural eligibility is not editorial approval.

This is a full-catalog mechanical triage plus selected semantic/editorial findings. It does not claim an individual semantic review of every term. Flags overlap and do not count accepted/rejected ingredients. No seed files, UUIDs, mapping approvals or database rows are changed.

| Review signal | Eligible terms | Interpretation |
|---|---:|---|
| `classification_label` | 5,999 | Classification scheme/code in label; select a culinary concept and review its wording. |
| `ontology_wording` | 1,154 | Ontology wording such as food product/material; inspect scope before shortening. |
| `animal_feed_branch` | 81 | Named ancestry includes food material for animals; check culinary use individually. |
| `hazard_category` | 13 | Named ancestry includes hazardous food material; an ontology category, not an ingredient name. |
| `generic_category` | 147 | Generic/organizational label; usually unsuitable as an initial suggestion. |
| `preparation_or_storage` | 4,351 | Preparation/storage variant; distinguish essential ingredient forms from recipe instructions. |
| `formulation_qualifier` | 1,407 | Composition/formulation qualifier; review whether it defines a useful ingredient. |
| `prepared_dish_word` | 687 | Prepared dish vocabulary; review ingredient use rather than excluding by keyword. |
| `special_diet_word` | 212 | Special-diet/formulation vocabulary; check scope. Baby vegetables are not matched. |
| `catchall_word` | 259 | Other/unspecified/catch-all wording needs scope review. |
| `canonical_text_invalid` | 7 | Proposed label/alias/description violates the canonical text contract. |
| `label_over_80_chars` | 133 | Long label; editorial review threshold, not a canonical validation limit. |
| `missing_definition` | 10,303 | No English definition; use other evidence before approving identity. |
| `shared_label_or_alias` | 587 | Normalized display name is shared by distinct source IDs; no automatic merge. |
| `existing_seed_name` | 42 | Name overlaps a maintained canonical label/alias; review mapping before creating an ID. |
| `editorial_family` | 8,894 | Mechanical simplification groups multiple labels; evidence of variants, not equivalence. |
| `manual_finding` | 54 | An explicit source-specific editorial recommendation is recorded. |

There are **346 shared normalized label/alias strings** across **587 terms**, despite **0 exact normalized label-duplicate groups**. Including aliases is essential.

Removing parentheses, numeric prefixes and 'food product' as a diagnostic groups **8,894 terms into 2,456 families**. Those families are deliberately lossy: raw/canned/dried, active/inactive, species and edible-part distinctions must survive when meaningful. Never use this simplification to merge identities.

**9,768** eligible terms have at least one attention flag other than preparation/family membership or missing definition alone. The remainder is not an approved clean set.

## Selected findings and recommendations

The 54 findings below are editorial proposals, not approved adapter import decisions. Suggested replacements require identity/scope review before ingestion.

| Source ID | Source label | Recommendation | Reason / proposed wording |
|---|---|---|---|
| FOODON_00001002 | food product | exclude_initial | Top-level processed consumable category includes human and animal products; not a specific ingredient. |
| FOODON_00001015 | plant food product | exclude_initial | Broad plant-derived product category; keep as taxonomy rather than a suggestion. |
| FOODON_00002454 | food material by characteristic | exclude_initial | Organizational category by characteristic. Do not exclude all descendants: many are legitimate ingredients. |
| FOODON_00002645 | food material by process | exclude_initial | Organizational category by process. Its fermented/pickled ingredient descendants need individual review. |
| FOODON_00002147 | food material by consumer group | exclude_initial | Organizational category by consumer group. |
| FOODON_00003366 | food (spoiled) | exclude_initial | Spoilage state of unspecified food rather than an ingredient identity. |
| FOODON_00005146 | hazardous food material | exclude_initial | Hazard classification rather than an ingredient identity; review its 13 structurally eligible branch terms separately from edible ingredients. |
| FOODON_00005152 | Hot food material which may trigger skin burns when handled without protective gloves. | exclude_initial | A sentence describing a burn hazard is stored as the label; not an ingredient name. |
| FOODON_00004325 | animal feed | exclude_initial | Explicit animal-feed category, not a human recipe ingredient. |
| FOODON_00002682 | pet food | exclude_initial | Explicit pet-food category; outside the initial human recipe seed. |
| FOODON_00004324 | compound feed premix (medicated) | exclude_initial | Medicated animal-feed premix; outside the initial human recipe seed. |
| FOODON_03301424 | bee pollen | review_scope | Counterexample to blanket feed-ancestry exclusion: bee pollen may have human culinary use, although this term descends from animal-feed material. Definition is absent; check the specific source identity. |
| FOODON_00000070 | lobster tail food product | edit_label | Replace ontology phrasing with a culinary label after checking whether the source covers meat, shell-on tails or tail products generally. Do not imply an exact meat-only scope automatically. Proposed label: lobster tail. |
| FOODON_00001025 | camembert cheese food product | edit_label | Remove ontology suffix after reconciling the other Camembert source term; preserve the curated spelling rather than creating parallel entries. Proposed label: Camembert cheese. |
| FOODON_00001042 | bovine milk beverage | edit_label | Prefer familiar animal terminology. Reconcile the bovine/cow milk branches and beverage-versus-ingredient scope first. Proposed label: cow’s milk. |
| FOODON_00001070 | miso food product | review_identity | A food-product category and a specific miso term both exist. Compare the broader rice/barley/soy fermentation definition with the soybean-based term before assigning mappings. |
| FOODON_00003208 | miso | review_identity | Specific miso definition overlaps the food-product term. Candidate for one reviewed culinary entry with appropriate source relationships, not an automatic string merge. |
| FOODON_03541304 | 13040 - cumin seed (efsa foodex2) | review_identity | The numeric prefix and classification suffix should not appear in the UI. Existing cumin-seed concepts already exist; shortening this label does not justify a new UUID or exact equivalence. Proposed label: cumin seed. |
| FOODON_03400113 | spice or herb (us cfr) | exclude_initial | A regulatory spice-or-herb umbrella category; stripping the source suffix would still leave a broad disjunction. |
| FOODON_03400343 | packaging gas (ec) | exclude_initial | Functional classification of packaging gases, not a specific cooking ingredient. |
| FOODON_00005799 | emulsifier | review_scope | Emulsifier is a functional role shared by different ingredients. Prefer named ingredients such as lecithin where evidence supports the specific identity. |
| FOODON_00003928 | pizza | defer_complete_dish | Pizza describes a prepared dish; lower priority for an initial ingredient seed. This is a scope recommendation, not an assertion that dishes can never be used as ingredients. |
| FOODON_00003930 | hawaiian pizza | defer_complete_dish | A named complete pizza variant would add suggestion noise in an ingredient list. |
| FOODON_03310321 | mushroom soup (canned) | consider_ingredient_use | Canned mushroom soup can be a recipe ingredient. A soup-keyword exclusion would lose useful pantry products; check identity and wording individually. Proposed label: canned mushroom soup. |
| FOODON_00002977 | baby spinach | retain_candidate | Baby spinach is a legitimate harvested vegetable stage. Do not treat the word baby as infant-formula evidence. |
| FOODON_00004437 | baby arugula greens | retain_candidate | Baby arugula is a legitimate vegetable stage; avoid broad baby-keyword exclusion. |
| FOODON_00001649 | black or white pepper product | review_aliases | Exact source alias pepper collides with Capsicum pepper and raw Capsicum pepper. Keep peppercorn seasoning distinct from the vegetable. |
| FOODON_00003520 | pepper | review_aliases | Definition identifies a Capsicum berry. Its label pepper collides with an exact alias for black/white peppercorn products; generic pepper must not merge these identities. |
| FOODON_00002667 | allspice | review_aliases | Exact alias pimento is also used for pimiento pepper. Allspice is Pimenta dioica fruit; preserve the regional ambiguity without equating the foods. |
| FOODON_00003484 | pimiento | review_aliases | Exact alias pimento overlaps allspice. Confirm the Capsicum/cherry-pepper identity and scope before canonical alias assignment. |
| FOODON_00001681 | caper food product | review_identity | Source definition includes caper flower buds OR fruit, whereas the caper child denotes buds. A shared caper alias does not establish identical scope. |
| FOODON_00003493 | caper | review_aliases | Source definition identifies an edible flower bud, but exact aliases Flinders rose and caper bush name the plant. Do not copy those plant-name synonyms automatically. |
| FOODON_00002465 | scone | review_aliases | Source labels this as scone and supplies biscuit as an exact alias. Regional biscuit terminology needs review alongside the hard-biscuit/cookie/cracker term. |
| FOODON_00002466 | biscuit (hard) food product | review_aliases | Source exact aliases biscuit, cookie and cracker cross regional/sweet/savoury distinctions. They should not automatically merge those canonical foods. |
| FOODON_00003717 | ginger root | review_identity | Definition says ginger root rather than rhizome; common culinary wording and botanical identity need reconciliation. Ginger also names multiple source scopes. |
| FOODON_00003752 | taro root | review_aliases | Definition says root rather than corm, and aliases include malanga, cocoyam and elephant’s ear. These names can cross plant/edible-part/species scopes; retain the pilot’s conservative mapping until resolved. |
| FOODON_00003441 | saffron thread | review_definition | Source definition says anther, whereas culinary saffron is the stigma. The pilot keeps related evidence; a friendlier label cannot repair the definition or establish exact equivalence. |
| FOODON_00003855 | tahini | retain_specific_scope | A clean-looking tahini label has a toasted-ground-hulled sesame definition. The pilot maps it narrower than generic tahini; preserve that semantic distinction. |
| FOODON_00002746 | soft-shell crab | repair_alias | An exact synonym is actually a 326-character explanatory paragraph. Keep the culinary label; quarantine the paragraph from aliases. The separate softshell crab alias can be reviewed normally. |
| FOODON_00001021 | acifified beer wort food product | hold_for_source_check | Likely source spelling error acifified. Acidified beer wort is a plausible correction, but confirm the intended identity and whether brewing intermediates belong in the initial seed. |
| FOODON_00001036 | pont ieveque cheese food product | hold_for_source_check | Likely misspelled Pont-l’Évêque cheese; its Normandy soft-cheese definition supports investigation. Confirm the name before promotion. Proposed label: Pont-l’Évêque cheese. |
| FOODON_00001048 | meshanger cheese food product | hold_for_source_check | Meshanger is unclear and has no English definition. Do not guess a replacement cheese name. |
| FOODON_03540200 | 02000 - yeast bread \u0096 pizza dough (efsa foodex2) | repair_text | Label contains U+0096 control/mojibake where punctuation was intended. It fails the canonical text contract; recover punctuation from source evidence before editing. |
| FOODON_03543540 | 35400 - shepherd\u0092s purse infusion leaves (efsa foodex2) | repair_text | Label contains U+0092 control/mojibake in the possessive. Repair text and review the classification term before proposing culinary wording. |
| FOODON_03400750 | 045  crustaceans (wc) (ccpr) | repair_text | The first proposed description is 2,164 characters, exceeding the 2,000-character canonical limit. It is also a broad regulatory crustaceans group; do not truncate into an approved ingredient. |
| FOODON_03302626 | yeast (fine, dry, active) | keep_distinct_form | Active dry yeast is functionally different from inactive yeast and fresh yeast. Preserve activity/form when choosing labels. Proposed label: active dry yeast. |
| FOODON_03310891 | yeast (fine, dry, inactive) | keep_distinct_form | Dry inactive yeast must not merge into active baking yeast. Check whether a specific nutritional-yeast identity is justified; inactivity alone does not establish it. Proposed label: inactive dry yeast. |
| FOODON_03315025 | yeast (fresh) | keep_distinct_form | Fresh yeast has different recipe handling and conversion from active dry yeast; preserve the form. Proposed label: fresh yeast. |
| FOODON_00005302 | tomato (sun-dried) | keep_distinct_form | Sun-dried tomatoes are a distinct pantry ingredient from fresh or canned tomatoes. Removing parentheses must not collapse them. Proposed label: sun-dried tomato. |
| FOODON_03301452 | tomato (canned) | keep_distinct_form | Canned tomato is a meaningful ingredient form. Whole/diced/puréed variants may warrant additional distinctions after review. Proposed label: canned tomato. |
| FOODON_03309927 | tomato (raw) | review_form_granularity | Raw tomato can be considered under a general culinary tomato entry only after scope review; no automatic equivalence to canned or dried forms. |
| FOODON_03301223 | paprika (ground) | retain_specific_scope | Definition identifies non-pungent ground paprika. Do not turn this into smoked paprika or infer preparation variants absent from the source. Proposed label: paprika. |
| FOODON_03304679 | squaw tea (dry) | hold_for_editorial_review | Source uses an outdated derogatory common name. Investigate botanical identity and a current culinary name before display; do not invent a replacement from the label alone. |
| FOODON_03414434 | 4-hexylresorcinol | review_scope | A real chemical name with a numeric locant. The adapter flags it as technical because it begins with a digit and hyphen; that is not evidence of a classification code. Review additive/culinary scope separately and preserve the numeric name. |

## Proposed initial-seed policy

- Keep specific culinary ingredients, including unfamiliar regional ingredients; unfamiliarity is not an exclusion reason.
- Hold organizational/regulatory categories, animal-specific products and hazard/state categories out of the initial suggestion list after checking actual scope. Feed ancestry alone is not sufficient: bee pollen also occurs there.
- Prefer reviewed culinary labels. Keep source codes/labels in provenance and mapping evidence. Removing a suffix does not establish exact equivalence.
- Resolve shared aliases and existing-seed matches before assigning a new canonical UUID. Do not merge peppercorns with Capsicum, allspice with pimiento, or caper buds with caper fruit.
- Preserve forms that change the ingredient: dried versus fresh yeast, active versus inactive yeast, dried versus canned tomatoes, powders/pastes/oils versus their source materials. Review incidental slicing/packaging/storage separately.
- Decide prepared-food scope by recipe use: bread and canned mushroom soup can be ingredients; named complete dishes are lower priority. Do not exclude all soup/salad/baby keywords.
- Repair invalid text and quarantine mistaken source synonyms. FoodOn exact-synonym annotations still require editorial checking.
- Preserve meaningful numbers and punctuation in names. For example, 4-hexylresorcinol is a chemical name, not a classification code; the adapter's broad technical-label warning is a review hint, not an exclusion rule.
- Preserve the existing pilot's broader/narrower/related judgments. Cosmetic label cleanup does not repair mismatched definitions or license an exact mapping.

## Reproduce

```bash
python3 scripts/vocabulary/audit_foodon_labels.py \
  --catalog /path/to/extracted/candidates.json \
  --output /tmp/noms-foodon-label-audit
```

The fresh output directory contains all eligible candidates in `candidates.csv`, shared strings in `shared-names.csv`, `summary.json`, input hashes in `inputs.json`, and this report. Raw catalogs and full candidate dumps stay outside Git. The source-specific findings are maintained in `data/vocabulary/adapters/foodon-label-review.json`.
