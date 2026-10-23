# International ingredient coverage review

Status: USDA review completed 2026-10-22 using the official bulk releases below. Wikidata and FoodOn judgments remain the 2026-10-21 review; they were not rediscovered in this pass.

**USDA supplies usable generic records for 27/100 ingredients, plus 34 additional branded-only ingredient products: 61/100 with either kind of USDA evidence.** The Wikidata/FoodOn union grows from 83 to 84 using generic USDA records, or 91 when branded-only product evidence is also allowed. These are observed discovery matches, not approved source mappings.

The benchmark retains the original 15 seed identities and adds 85 international ingredients across 19 illustrative usage strata. This is a purposive long-tail stress test, not a random or frequency-weighted estimate of all ingredients worldwide. Strata contain only 2–15 items and do not represent entire cuisines or exclusive origins.

## Match rules

- **E**: usable ingredient concept or generic food record preserving the scoped identity, part and required form. Compatible raw/dried or whole/ground presentations may qualify when identity is established. This is not formal exact ontology equivalence or permission to import unchanged.
- **B**: usable USDA branded-only ingredient product; no generic E was confirmed. This is product evidence, kept separate from generic concept coverage.
- **R**: a reviewed broader, ambiguous or differently processed food/material/product; excluded from usable totals.
- **N**: no scoped match confirmed by the saved searches and review in the specified release. This is not proof of absence under every possible synonym or future release.
- **—** in the additional-product column: not hand-graded because generic USDA coverage already exists. Branded coverage for all 100 ingredients is not asserted.

Normal formulation ingredients are permitted for a benchmark that is itself a named preparation, such as gochujang, harissa or ras el hanout. A meal, unrelated seasoning blend, flavored product, or ingredient-list mention alone does not count for a constituent ingredient. Required transformations and material distinctions remain significant: smoked paprika, dry masa harina, leaf versus seed, garlic versus garlic paste, and regionally distinct fermented shrimp preparations.

## Method and pinned evidence

USDA archives were downloaded from the official download page on 2026-10-21 over the inherited proxy with TLS verification. The updated runtime reports unrestricted, enforced HTTP policy. The bulk download blocker is resolved; API access was not reverified and no personal API key or DEMO_KEY search quota was needed. Archive URLs, exact byte lengths, SHA-256 values and ZIP member identities are in `source-snapshots.json`.

| USDA data type | Release | Eligible food records | Observed usable benchmark matches |
|---|---|---:|---:|
| Foundation | [2026-04-30](https://fdc.nal.usda.gov/fdc-datasets/FoodData_Central_foundation_food_csv_2026-04-30.zip) | 469 | 10/100 |
| SR Legacy | [2018-04](https://fdc.nal.usda.gov/fdc-datasets/FoodData_Central_sr_legacy_food_csv_2018-04.zip) | 7,793 | 26/100 |
| FNDDS | [2024-10-31](https://fdc.nal.usda.gov/fdc-datasets/FoodData_Central_survey_food_csv_2024-10-31.zip) | 5,432 | 10/100 |
| Branded | [2026-04-30](https://fdc.nal.usda.gov/fdc-datasets/FoodData_Central_branded_food_csv_2026-04-30.zip) | 1,999,950 | 34 additional / 73 generic gaps |

Each complete `food.csv` was streamed without extracting nutrition tables. Only `foundation_food`, `sr_legacy_food`, `survey_fndds_food` and `branded_food` rows are eligible; Foundation sample and acquisition rows are excluded from generic coverage. Description searches use saved labels, aliases and fallback phrases with case/diacritic folding and word boundaries. Generic fallbacks include USDA naming variants such as pilinuts and drumstick leaves. Searches produce candidates; identity/part/form judgments are manual.

For the 73 generic gaps, branded candidate descriptions were joined to `branded_food.csv` for ingredient and product checks. Ingredient-list-only mentions were not used as independent ingredient records or as a reverse ingredient-list search. `usda-review.json` retains all queries, judgments, selected FDC IDs, exact descriptions, data types, releases, URLs, reasons and selected product ingredients. It retains representative evidence, not every duplicate nutritional/product version. The targeted branded pass supports an incremental count, not a standalone branded-completeness percentage.

Wikidata remains an observed lower bound from indexed search, public entity pages, redirects, sitelinks and plant-to-food relation traversal. Its API was unavailable in the earlier review; cached representations were not an atomic snapshot. FoodOn was reviewed from the full pinned OWL snapshot, including definitions/synonyms and specific FoodEx2 categories. Because source representations and discovery methods differ, this is an observed-coverage comparison, not a definitive completeness ranking.

## Observed coverage

| Source / pass | Usable | Related R | No scoped match confirmed N | Not assessed |
|---|---:|---:|---:|---:|
| Wikidata | 68/100 E | 32 | 0 | 0 |
| FoodOn | 50/100 E | 30 | 20 | 0 |
| USDA generic union | 27/100 E | 32 | 41 | 0 |
| USDA additional products | 34 B | 16 | 23 | 27 already generic-covered |

Foundation, SR Legacy and FNDDS overlap; their usable counts must not be added. The generic union is 27/100. Adding the 34 branded-only products gives 61/100 observed USDA coverage; 39 ingredients remain related-only or without a confirmed match.

USDA generic evidence fills 1 Wikidata/FoodOn gap: masa harina. Additional product evidence fills 7 more: smoked paprika, gochugaru, white miso, Chinese black vinegar, belacan, bagoong alamang, ackee. The resulting three-source discovery union is **84/100 with generic USDA evidence**, or **91/100 including product evidence**. A product record does not turn the latter into generic ontology coverage.

## Coverage by sampling stratum

| Stratum | Items | Wikidata E | FoodOn E | USDA generic E | USDA additional B | Concept/generic union | Including products |
|---|---:|---:|---:|---:|---:|---:|---:|
| Existing seed controls | 15 | 12 | 13 | 9 | 5 | 14 | 15 |
| Korean | 5 | 4 | 0 | 0 | 2 | 4 | 5 |
| Japanese | 5 | 4 | 1 | 0 | 4 | 4 | 5 |
| Chinese and Taiwanese | 5 | 4 | 1 | 0 | 2 | 4 | 5 |
| Mainland Southeast Asian | 5 | 2 | 4 | 1 | 0 | 5 | 5 |
| Maritime Southeast Asian | 5 | 3 | 2 | 1 | 1 | 3 | 4 |
| Filipino | 5 | 2 | 3 | 1 | 4 | 4 | 5 |
| South Asian | 5 | 2 | 2 | 0 | 2 | 3 | 3 |
| Iranian Central Asian and Caucasian | 5 | 3 | 0 | 0 | 0 | 3 | 3 |
| Levantine and Anatolian | 5 | 5 | 2 | 1 | 3 | 5 | 5 |
| North African | 5 | 4 | 2 | 0 | 3 | 4 | 4 |
| Horn of Africa | 5 | 4 | 1 | 1 | 2 | 4 | 4 |
| West African | 5 | 4 | 3 | 1 | 1 | 5 | 5 |
| Eastern and Southern African | 5 | 0 | 4 | 3 | 0 | 4 | 4 |
| Mexican and Central American | 5 | 2 | 3 | 2 | 2 | 5 | 5 |
| Andean and Amazonian | 5 | 4 | 1 | 0 | 2 | 4 | 4 |
| European | 5 | 5 | 5 | 4 | 0 | 5 | 5 |
| Caribbean | 3 | 2 | 1 | 1 | 1 | 2 | 3 |
| Pacific | 2 | 2 | 2 | 2 | 0 | 2 | 2 |

## Findings and limits

Wikidata contributes named regional fermented foods and seasoning blends. FoodOn contributes edible-part and processed-material distinctions, including garlic paste, black sesame, greater galangal rhizome, culantro leaves, annatto seed, baobab powder, moringa leaves and Bambara groundnut. USDA adds useful generic food forms, including explicit dry masa harina, but most of its incremental regional coverage in this benchmark comes from branded products.

Specific USDA safeguards: unsmoked paprika and unspecified miso remain R for smoked paprika and white miso until a named product supplies the missing form. Generic fermented shrimp sauce does not establish belacan or saeujeot. Belacan is supported separately by the named BLACHEN dried shrimp paste product, and bagoong alamang has its own named product; neither is inferred solely from generic shrimp-paste synonyms. Galangal powder lacks greater-galangal species/part evidence; bonito flakes do not explicitly confirm the required smoking/fermentation. Salted sumac, huacatay paste, prepared huitlacoche, lingonberry preserves and Scotch bonnet sauces are retained as related products under the scoped ingredient rules. These conservative judgments can be revisited with stronger evidence; they are not claims that the foods are unavailable in commerce.

Reject lexical traps: CHUNO PURE chocolate is not freeze-dried potato, li-hing snacks are not asafoetida, pimento sweet pepper is not allspice, and a Scotch bonnet mushroom is not a pepper. Do not merge belacan with bagoong merely through a generic shrimp-paste synonym, Korean chili pepper with gochugaru, or wet masa dough with dry masa harina. Resolve Wikidata plant/food distinctions and redirects before mapping canonical ingredients.

USDA branded data are supplier-submitted product records and can be historical, duplicated or poorly labeled; a release date is not a product publication date or a guarantee of current availability. Selected positives have no supplied discontinuation date, but that does not establish present stock. The niter-kibbeh record has an unexpected brand-owner value, illustrating why these are research candidates rather than import-ready authority. Product ingredients were reviewed for identity, not audited for authenticity or nutritional interchangeability.

The 9 remaining observed union gaps, even allowing product evidence, are: torch ginger flower, kokum, goraka, blue fenugreek, kashk, bsissa, korarima, mopane worm, huacatay. Related records may help deeper curation, but do not preserve the required scoped identity in this review.

## Implications for Noms

Keep the curated seed as the authority. Use Wikidata for regional names and preparations, FoodOn for material/part/form distinctions, and generic USDA records for food/nutrition-linked evidence. Use USDA products as supplemental discovery leads with explicit product status; they do not replace a concept vocabulary. Preserve Noms canonical IDs and review external mappings separately. No source mappings were promoted and no canonical ingredient data were changed by this research.

## Validation and regeneration

Run `python3 data/vocabulary/coverage/summarize.py` to validate alignment and regenerate this report and `coverage.csv`. It does not query sources or discover matches. To audit the retained USDA evidence, download the pinned archives to the `local_filename` names and run `python3 data/vocabulary/coverage/inspect_usda.py --archives DIR`. It verifies SHA-256, ZIP members, eligible record counts, exact selected descriptions/publication dates and selected product metadata/ingredients while rerunning the saved description searches. Optional `--candidates PATH` writes a generated candidate CSV outside the checkout; judgments are never automatically assigned.

This update changes research artifacts and their analysis scripts. It does not claim a new application-wide test run. The separate local app startup loaded the existing 100-entry seed; the source investigation itself performs no database writes.

## Ingredient matrix

Links select representative evidence. JSON/CSV retain all selected generic data-type representatives and full product evidence. An E in the additional-product column contributes B to the USDA combined status.

| # | Ingredient | Stratum | Wikidata | FoodOn | USDA generic | USDA additional product |
|---:|---|---|---|---|---|---|
| 1 | chickpea | Existing seed controls | [E](https://www.wikidata.org/wiki/Q21156930) | [E](http://purl.obolibrary.org/obo/FOODON_03306811) | [E](https://fdc.nal.usda.gov/food-details/2644282/nutrients) | — |
| 2 | gochujang | Existing seed controls | [E](https://www.wikidata.org/wiki/Q699637) | N | N | [B](https://fdc.nal.usda.gov/food-details/1108244/nutrients) |
| 3 | garlic | Existing seed controls | [E](https://www.wikidata.org/wiki/Q21546392) | [E](http://purl.obolibrary.org/obo/FOODON_00003582) | [E](https://fdc.nal.usda.gov/food-details/1104647/nutrients) | — |
| 4 | garlic powder | Existing seed controls | [E](https://www.wikidata.org/wiki/Q10716334) | [E](http://purl.obolibrary.org/obo/FOODON_03301844) | [E](https://fdc.nal.usda.gov/food-details/171325/nutrients) | — |
| 5 | garlic paste | Existing seed controls | [R](https://www.wikidata.org/wiki/Q21546392) | [E](http://purl.obolibrary.org/obo/FOODON_03305217) | [R](https://fdc.nal.usda.gov/food-details/169230/nutrients) | [B](https://fdc.nal.usda.gov/food-details/1889044/nutrients) |
| 6 | chili pepper | Existing seed controls | [E](https://www.wikidata.org/wiki/Q165199) | [E](http://purl.obolibrary.org/obo/FOODON_00003744) | [E](https://fdc.nal.usda.gov/food-details/170106/nutrients) | — |
| 7 | tomato | Existing seed controls | [E](https://www.wikidata.org/wiki/Q20638126) | [E](http://purl.obolibrary.org/obo/FOODON_03301453) | [E](https://fdc.nal.usda.gov/food-details/321360/nutrients) | — |
| 8 | olive oil | Existing seed controls | [E](https://www.wikidata.org/wiki/Q93165) | [E](http://purl.obolibrary.org/obo/FOODON_03301826) | [E](https://fdc.nal.usda.gov/food-details/748608/nutrients) | — |
| 9 | soy sauce | Existing seed controls | [E](https://www.wikidata.org/wiki/Q229385) | [E](http://purl.obolibrary.org/obo/FOODON_03544111) | [E](https://fdc.nal.usda.gov/food-details/174277/nutrients) | — |
| 10 | salt | Existing seed controls | [E](https://www.wikidata.org/wiki/Q11254) | [E](http://purl.obolibrary.org/obo/FOODON_00002250) | [E](https://fdc.nal.usda.gov/food-details/746775/nutrients) | — |
| 11 | tahini | Existing seed controls | [E](https://www.wikidata.org/wiki/Q383389) | [E](http://purl.obolibrary.org/obo/FOODON_00003855) | [E](https://fdc.nal.usda.gov/food-details/2262073/nutrients) | — |
| 12 | sumac | Existing seed controls | [E](https://www.wikidata.org/wiki/Q16131953) | [E](http://purl.obolibrary.org/obo/FOODON_03541395) | N | [R](https://fdc.nal.usda.gov/food-details/2630657/nutrients) |
| 13 | rice vinegar | Existing seed controls | [E](https://www.wikidata.org/wiki/Q1849012) | [E](http://purl.obolibrary.org/obo/FOODON_03307370) | [R](https://fdc.nal.usda.gov/food-details/2709351/nutrients) | [B](https://fdc.nal.usda.gov/food-details/2576812/nutrients) |
| 14 | smoked paprika | Existing seed controls | [R](https://www.wikidata.org/wiki/Q3127593) | [R](http://purl.obolibrary.org/obo/FOODON_03301223) | [R](https://fdc.nal.usda.gov/food-details/171329/nutrients) | [B](https://fdc.nal.usda.gov/food-details/2060565/nutrients) |
| 15 | black sesame seeds | Existing seed controls | [R](https://www.wikidata.org/wiki/Q12000036) | [E](http://purl.obolibrary.org/obo/FOODON_00004782) | [R](https://fdc.nal.usda.gov/food-details/170150/nutrients) | [B](https://fdc.nal.usda.gov/food-details/2138385/nutrients) |
| 16 | doenjang | Korean | [E](https://www.wikidata.org/wiki/Q1196289) | [R](http://purl.obolibrary.org/obo/FOODON_03310005) | [R](https://fdc.nal.usda.gov/food-details/172442/nutrients) | [B](https://fdc.nal.usda.gov/food-details/2040094/nutrients) |
| 17 | gochugaru | Korean | [R](https://www.wikidata.org/wiki/Q48189338) | [R](http://purl.obolibrary.org/obo/FOODON_03302030) | [R](https://fdc.nal.usda.gov/food-details/170106/nutrients) | [B](https://fdc.nal.usda.gov/food-details/2084510/nutrients) |
| 18 | perilla leaf | Korean | [E](https://www.wikidata.org/wiki/Q27650144) | [R](http://purl.obolibrary.org/obo/FOODON_03412919) | N | [R](https://fdc.nal.usda.gov/food-details/2628317/nutrients) |
| 19 | saeujeot | Korean | [E](https://www.wikidata.org/wiki/Q843252) | [R](http://purl.obolibrary.org/obo/FOODON_03309823) | N | [R](https://fdc.nal.usda.gov/food-details/2024889/nutrients) |
| 20 | cheonggukjang | Korean | [E](https://www.wikidata.org/wiki/Q1071614) | [R](http://purl.obolibrary.org/obo/FOODON_00001078) | [R](https://fdc.nal.usda.gov/food-details/172442/nutrients) | N |
| 21 | white miso | Japanese | [R](https://www.wikidata.org/wiki/Q235169) | [R](http://purl.obolibrary.org/obo/FOODON_00003208) | [R](https://fdc.nal.usda.gov/food-details/172442/nutrients) | [B](https://fdc.nal.usda.gov/food-details/2551468/nutrients) |
| 22 | kombu | Japanese | [E](https://www.wikidata.org/wiki/Q11261547) | [E](http://purl.obolibrary.org/obo/FOODON_03317000) | [R](https://fdc.nal.usda.gov/food-details/168457/nutrients) | [B](https://fdc.nal.usda.gov/food-details/1862743/nutrients) |
| 23 | katsuobushi | Japanese | [E](https://www.wikidata.org/wiki/Q113739) | [R](http://purl.obolibrary.org/obo/FOODON_03311594) | N | [R](https://fdc.nal.usda.gov/food-details/2025412/nutrients) |
| 24 | shio koji | Japanese | [E](https://www.wikidata.org/wiki/Q11428977) | [R](http://purl.obolibrary.org/obo/FOODON_03311656) | N | [B](https://fdc.nal.usda.gov/food-details/2027677/nutrients) |
| 25 | umeboshi | Japanese | [E](https://www.wikidata.org/wiki/Q1045417) | [R](http://purl.obolibrary.org/obo/FOODON_03541493) | N | [B](https://fdc.nal.usda.gov/food-details/2675009/nutrients) |
| 26 | doubanjiang | Chinese and Taiwanese | [E](https://www.wikidata.org/wiki/Q3273096) | [R](http://purl.obolibrary.org/obo/FOODON_03310005) | N | [R](https://fdc.nal.usda.gov/food-details/2603243/nutrients) |
| 27 | douchi | Chinese and Taiwanese | [E](https://www.wikidata.org/wiki/Q3242685) | [R](http://purl.obolibrary.org/obo/FOODON_00001078) | [R](https://fdc.nal.usda.gov/food-details/174270/nutrients) | [R](https://fdc.nal.usda.gov/food-details/2041303/nutrients) |
| 28 | Shaoxing wine | Chinese and Taiwanese | [E](https://www.wikidata.org/wiki/Q175245) | [R](http://purl.obolibrary.org/obo/FOODON_03307312) | N | [B](https://fdc.nal.usda.gov/food-details/2440010/nutrients) |
| 29 | Sichuan pepper | Chinese and Taiwanese | [E](https://www.wikidata.org/wiki/Q756800) | [E](http://purl.obolibrary.org/obo/FOODON_00003470) | N | [R](https://fdc.nal.usda.gov/food-details/2626697/nutrients) |
| 30 | Chinese black vinegar | Chinese and Taiwanese | [R](https://www.wikidata.org/wiki/Q4922497) | [R](http://purl.obolibrary.org/obo/FOODON_03307370) | [R](https://fdc.nal.usda.gov/food-details/2709351/nutrients) | [B](https://fdc.nal.usda.gov/food-details/2436644/nutrients) |
| 31 | galangal | Mainland Southeast Asian | [R](https://www.wikidata.org/wiki/Q734727) | [E](http://purl.obolibrary.org/obo/FOODON_03541357) | N | [R](https://fdc.nal.usda.gov/food-details/399808/nutrients) |
| 32 | makrut lime leaf | Mainland Southeast Asian | [E](https://www.wikidata.org/wiki/Q67202802) | [E](http://purl.obolibrary.org/obo/FOODON_03540903) | N | [R](https://fdc.nal.usda.gov/food-details/2110541/nutrients) |
| 33 | lemongrass | Mainland Southeast Asian | [R](https://www.wikidata.org/wiki/Q33913) | [E](http://purl.obolibrary.org/obo/FOODON_00003340) | [E](https://fdc.nal.usda.gov/food-details/168573/nutrients) | — |
| 34 | culantro | Mainland Southeast Asian | [R](https://www.wikidata.org/wiki/Q1146782) | [E](http://purl.obolibrary.org/obo/FOODON_03540838) | N | [R](https://fdc.nal.usda.gov/food-details/1868971/nutrients) |
| 35 | padaek | Mainland Southeast Asian | [E](https://www.wikidata.org/wiki/Q6853367) | [R](http://purl.obolibrary.org/obo/FOODON_03542615) | [R](https://fdc.nal.usda.gov/food-details/2706457/nutrients) | N |
| 36 | belacan | Maritime Southeast Asian | [R](https://www.wikidata.org/wiki/Q1127705) | [R](http://purl.obolibrary.org/obo/FOODON_03309823) | N | [B](https://fdc.nal.usda.gov/food-details/2032323/nutrients) |
| 37 | candlenut | Maritime Southeast Asian | [E](https://www.wikidata.org/wiki/Q104821775) | [E](http://purl.obolibrary.org/obo/FOODON_03541316) | N | N |
| 38 | tempeh | Maritime Southeast Asian | [E](https://www.wikidata.org/wiki/Q120096) | [E](http://purl.obolibrary.org/obo/FOODON_03304999) | [E](https://fdc.nal.usda.gov/food-details/174272/nutrients) | — |
| 39 | buah keluak | Maritime Southeast Asian | [E](https://www.wikidata.org/wiki/Q12512536) | [R](http://purl.obolibrary.org/obo/FOODON_00004311) | N | N |
| 40 | torch ginger flower | Maritime Southeast Asian | [R](https://www.wikidata.org/wiki/Q3442435) | N | N | N |
| 41 | bagoong alamang | Filipino | [R](https://www.wikidata.org/wiki/Q1127705) | [R](http://purl.obolibrary.org/obo/FOODON_03309823) | N | [B](https://fdc.nal.usda.gov/food-details/2032276/nutrients) |
| 42 | calamansi | Filipino | [R](https://www.wikidata.org/wiki/Q849387) | [E](http://purl.obolibrary.org/obo/FOODON_03541458) | N | [B](https://fdc.nal.usda.gov/food-details/799617/nutrients) |
| 43 | annatto seed | Filipino | [R](https://www.wikidata.org/wiki/Q425902) | [E](http://purl.obolibrary.org/obo/FOODON_03000015) | [R](https://fdc.nal.usda.gov/food-details/172242/nutrients) | [B](https://fdc.nal.usda.gov/food-details/2655209/nutrients) |
| 44 | banana ketchup | Filipino | [E](https://www.wikidata.org/wiki/Q3907811) | N | N | [B](https://fdc.nal.usda.gov/food-details/2613005/nutrients) |
| 45 | pili nut | Filipino | [E](https://www.wikidata.org/wiki/Q104188819) | [E](http://purl.obolibrary.org/obo/FOODON_00003722) | [E](https://fdc.nal.usda.gov/food-details/170590/nutrients) | — |
| 46 | asafoetida | South Asian | [E](https://www.wikidata.org/wiki/Q16530385) | [E](http://purl.obolibrary.org/obo/FOODON_00002936) | N | N |
| 47 | ajwain | South Asian | [R](https://www.wikidata.org/wiki/Q413254) | [E](http://purl.obolibrary.org/obo/FOODON_03541377) | N | [B](https://fdc.nal.usda.gov/food-details/1126916/nutrients) |
| 48 | amchur | South Asian | [E](https://www.wikidata.org/wiki/Q2015681) | N | [R](https://fdc.nal.usda.gov/food-details/169910/nutrients) | [B](https://fdc.nal.usda.gov/food-details/2428551/nutrients) |
| 49 | kokum | South Asian | [R](https://www.wikidata.org/wiki/Q2095514) | [R](http://purl.obolibrary.org/obo/FOODON_03412803) | N | N |
| 50 | goraka | South Asian | [R](https://www.wikidata.org/wiki/Q18187249) | [R](http://purl.obolibrary.org/obo/FOODON_03412780) | N | [R](https://fdc.nal.usda.gov/food-details/2386349/nutrients) |
| 51 | dried lime | Iranian Central Asian and Caucasian | [E](https://www.wikidata.org/wiki/Q460570) | [R](http://purl.obolibrary.org/obo/FOODON_00003661) | [R](https://fdc.nal.usda.gov/food-details/168155/nutrients) | [R](https://fdc.nal.usda.gov/food-details/2507191/nutrients) |
| 52 | barberry | Iranian Central Asian and Caucasian | [E](https://www.wikidata.org/wiki/Q139555604) | [R](http://purl.obolibrary.org/obo/FOODON_03412492) | N | N |
| 53 | blue fenugreek | Iranian Central Asian and Caucasian | [R](https://www.wikidata.org/wiki/Q164849) | [R](http://purl.obolibrary.org/obo/FOODON_03415082) | [R](https://fdc.nal.usda.gov/food-details/171324/nutrients) | N |
| 54 | tklapi | Iranian Central Asian and Caucasian | [E](https://www.wikidata.org/wiki/Q2906012) | N | [R](https://fdc.nal.usda.gov/food-details/167541/nutrients) | N |
| 55 | kashk | Iranian Central Asian and Caucasian | [R](https://www.wikidata.org/wiki/Q1113301) | N | [R](https://fdc.nal.usda.gov/food-details/171281/nutrients) | N |
| 56 | bulgur | Levantine and Anatolian | [E](https://www.wikidata.org/wiki/Q250432) | [E](http://purl.obolibrary.org/obo/FOODON_00003143) | [E](https://fdc.nal.usda.gov/food-details/2710820/nutrients) | — |
| 57 | freekeh | Levantine and Anatolian | [E](https://www.wikidata.org/wiki/Q771079) | [E](http://purl.obolibrary.org/obo/FOODON_00004577) | [R](https://fdc.nal.usda.gov/food-details/169721/nutrients) | [B](https://fdc.nal.usda.gov/food-details/2020470/nutrients) |
| 58 | pomegranate molasses | Levantine and Anatolian | [E](https://www.wikidata.org/wiki/Q108946643) | [R](http://purl.obolibrary.org/obo/FOODON_03543248) | [R](https://fdc.nal.usda.gov/food-details/167787/nutrients) | [B](https://fdc.nal.usda.gov/food-details/2532231/nutrients) |
| 59 | mahlab | Levantine and Anatolian | [E](https://www.wikidata.org/wiki/Q465554) | N | N | N |
| 60 | Aleppo pepper | Levantine and Anatolian | [E](https://www.wikidata.org/wiki/Q486606) | [R](http://purl.obolibrary.org/obo/FOODON_00003744) | [R](https://fdc.nal.usda.gov/food-details/168579/nutrients) | [B](https://fdc.nal.usda.gov/food-details/512762/nutrients) |
| 61 | harissa | North African | [E](https://www.wikidata.org/wiki/Q966327) | [E](http://purl.obolibrary.org/obo/FOODON_00002484) | N | [B](https://fdc.nal.usda.gov/food-details/2304490/nutrients) |
| 62 | preserved lemon | North African | [E](https://www.wikidata.org/wiki/Q2723408) | [E](http://purl.obolibrary.org/obo/FOODON_03316981) | [R](https://fdc.nal.usda.gov/food-details/167746/nutrients) | [B](https://fdc.nal.usda.gov/food-details/2076581/nutrients) |
| 63 | ras el hanout | North African | [E](https://www.wikidata.org/wiki/Q1067261) | N | N | [B](https://fdc.nal.usda.gov/food-details/2273152/nutrients) |
| 64 | smen | North African | [E](https://www.wikidata.org/wiki/Q2397413) | N | [R](https://fdc.nal.usda.gov/food-details/173410/nutrients) | N |
| 65 | bsissa | North African | [R](https://www.wikidata.org/wiki/Q2527136) | N | N | N |
| 66 | teff | Horn of Africa | [E](https://www.wikidata.org/wiki/Q103205493) | [E](http://purl.obolibrary.org/obo/FOODON_03000255) | [E](https://fdc.nal.usda.gov/food-details/169747/nutrients) | — |
| 67 | berbere | Horn of Africa | [E](https://www.wikidata.org/wiki/Q818763) | N | N | [B](https://fdc.nal.usda.gov/food-details/2626237/nutrients) |
| 68 | mitmita | Horn of Africa | [E](https://www.wikidata.org/wiki/Q1762278) | N | N | N |
| 69 | niter kibbeh | Horn of Africa | [E](https://www.wikidata.org/wiki/Q2697883) | N | [R](https://fdc.nal.usda.gov/food-details/173412/nutrients) | [B](https://fdc.nal.usda.gov/food-details/2174118/nutrients) |
| 70 | korarima | Horn of Africa | [R](https://www.wikidata.org/wiki/Q4689432) | N | [R](https://fdc.nal.usda.gov/food-details/170919/nutrients) | N |
| 71 | egusi | West African | [E](https://www.wikidata.org/wiki/Q10771593) | [E](http://purl.obolibrary.org/obo/FOODON_00003899) | [R](https://fdc.nal.usda.gov/food-details/169407/nutrients) | [B](https://fdc.nal.usda.gov/food-details/2114492/nutrients) |
| 72 | iru | West African | [E](https://www.wikidata.org/wiki/Q137835655) | N | N | N |
| 73 | ogiri | West African | [E](https://www.wikidata.org/wiki/Q7080074) | N | N | N |
| 74 | grains of paradise | West African | [E](https://www.wikidata.org/wiki/Q84574093) | [E](http://purl.obolibrary.org/obo/FOODON_03305445) | N | N |
| 75 | fonio | West African | [R](https://www.wikidata.org/wiki/Q12439809) | [E](http://purl.obolibrary.org/obo/FOODON_03540027) | [E](https://fdc.nal.usda.gov/food-details/2710829/nutrients) | — |
| 76 | baobab fruit powder | Eastern and Southern African | [R](https://www.wikidata.org/wiki/Q3360594) | [E](http://purl.obolibrary.org/obo/FOODON_00005549) | [E](https://fdc.nal.usda.gov/food-details/167806/nutrients) | — |
| 77 | mopane worm | Eastern and Southern African | [R](https://www.wikidata.org/wiki/Q311788) | N | N | N |
| 78 | amaranth leaf | Eastern and Southern African | [R](https://www.wikidata.org/wiki/Q156344) | [E](http://purl.obolibrary.org/obo/FOODON_03000026) | [E](https://fdc.nal.usda.gov/food-details/168385/nutrients) | — |
| 79 | moringa leaf | Eastern and Southern African | [R](https://www.wikidata.org/wiki/Q234193) | [E](http://purl.obolibrary.org/obo/FOODON_00005327) | [E](https://fdc.nal.usda.gov/food-details/168416/nutrients) | — |
| 80 | Bambara groundnut | Eastern and Southern African | [R](https://www.wikidata.org/wiki/Q338219) | [E](http://purl.obolibrary.org/obo/FOODON_00002775) | N | N |
| 81 | masa harina | Mexican and Central American | [R](https://www.wikidata.org/wiki/Q1006089) | [R](http://purl.obolibrary.org/obo/FOODON_00002529) | [E](https://fdc.nal.usda.gov/food-details/2710835/nutrients) | — |
| 82 | epazote | Mexican and Central American | [R](https://www.wikidata.org/wiki/Q640032) | [E](http://purl.obolibrary.org/obo/FOODON_03540909) | [E](https://fdc.nal.usda.gov/food-details/169398/nutrients) | — |
| 83 | huitlacoche | Mexican and Central American | [R](https://www.wikidata.org/wiki/Q1430040) | [E](http://purl.obolibrary.org/obo/FOODON_03540785) | N | [R](https://fdc.nal.usda.gov/food-details/2673786/nutrients) |
| 84 | achiote paste | Mexican and Central American | [E](https://www.wikidata.org/wiki/Q7302262) | N | [R](https://fdc.nal.usda.gov/food-details/172242/nutrients) | [B](https://fdc.nal.usda.gov/food-details/931916/nutrients) |
| 85 | chipotle pepper | Mexican and Central American | [E](https://www.wikidata.org/wiki/Q860808) | [E](http://purl.obolibrary.org/obo/FOODON_00002563) | [R](https://fdc.nal.usda.gov/food-details/168576/nutrients) | [B](https://fdc.nal.usda.gov/food-details/1908017/nutrients) |
| 86 | aji amarillo | Andean and Amazonian | [E](https://www.wikidata.org/wiki/Q61413777) | [R](http://purl.obolibrary.org/obo/FOODON_03540653) | [R](https://fdc.nal.usda.gov/food-details/170106/nutrients) | [B](https://fdc.nal.usda.gov/food-details/1548985/nutrients) |
| 87 | huacatay | Andean and Amazonian | [R](https://www.wikidata.org/wiki/Q599870) | [R](http://purl.obolibrary.org/obo/FOODON_03414999) | N | [R](https://fdc.nal.usda.gov/food-details/2669225/nutrients) |
| 88 | chuño | Andean and Amazonian | [E](https://www.wikidata.org/wiki/Q2352129) | N | [R](https://fdc.nal.usda.gov/food-details/170445/nutrients) | N |
| 89 | rocoto pepper | Andean and Amazonian | [E](https://www.wikidata.org/wiki/Q104006555) | [E](http://purl.obolibrary.org/obo/FOODON_03540656) | [R](https://fdc.nal.usda.gov/food-details/170106/nutrients) | [B](https://fdc.nal.usda.gov/food-details/2669220/nutrients) |
| 90 | tucupi | Andean and Amazonian | [E](https://www.wikidata.org/wiki/Q491222) | N | N | N |
| 91 | saffron | European | [E](https://www.wikidata.org/wiki/Q25434) | [E](http://purl.obolibrary.org/obo/FOODON_00003441) | [E](https://fdc.nal.usda.gov/food-details/170934/nutrients) | — |
| 92 | caper | European | [E](https://www.wikidata.org/wiki/Q3229775) | [E](http://purl.obolibrary.org/obo/FOODON_00003493) | [E](https://fdc.nal.usda.gov/food-details/172238/nutrients) | — |
| 93 | caraway seed | European | [E](https://www.wikidata.org/wiki/Q21155816) | [E](http://purl.obolibrary.org/obo/FOODON_00003023) | [E](https://fdc.nal.usda.gov/food-details/170918/nutrients) | — |
| 94 | rye flour | European | [E](https://www.wikidata.org/wiki/Q10658708) | [E](http://purl.obolibrary.org/obo/FOODON_03302492) | [E](https://fdc.nal.usda.gov/food-details/2512375/nutrients) | — |
| 95 | lingonberry | European | [E](https://www.wikidata.org/wiki/Q12339956) | [E](http://purl.obolibrary.org/obo/FOODON_00003542) | N | [R](https://fdc.nal.usda.gov/food-details/2025975/nutrients) |
| 96 | ackee | Caribbean | [R](https://www.wikidata.org/wiki/Q102828166) | [R](http://purl.obolibrary.org/obo/FOODON_00003489) | N | [B](https://fdc.nal.usda.gov/food-details/2655338/nutrients) |
| 97 | allspice | Caribbean | [E](https://www.wikidata.org/wiki/Q1963621) | [E](http://purl.obolibrary.org/obo/FOODON_00002667) | [E](https://fdc.nal.usda.gov/food-details/171315/nutrients) | — |
| 98 | Scotch bonnet pepper | Caribbean | [E](https://www.wikidata.org/wiki/Q466854) | [R](http://purl.obolibrary.org/obo/FOODON_03412555) | [R](https://fdc.nal.usda.gov/food-details/170106/nutrients) | [R](https://fdc.nal.usda.gov/food-details/2204172/nutrients) |
| 99 | taro | Pacific | [E](https://www.wikidata.org/wiki/Q134452914) | [E](http://purl.obolibrary.org/obo/FOODON_00003752) | [E](https://fdc.nal.usda.gov/food-details/169308/nutrients) | — |
| 100 | coconut milk | Pacific | [E](https://www.wikidata.org/wiki/Q841779) | [E](http://purl.obolibrary.org/obo/FOODON_03303084) | [E](https://fdc.nal.usda.gov/food-details/170173/nutrients) | — |
