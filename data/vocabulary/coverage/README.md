# International ingredient coverage review

Status: interim. Wikidata and FoodOn have been reviewed; USDA bulk/API access is blocked by the current cloud network policy. No USDA coverage percentage is asserted.

Reviewed 2026-10-21. The seed contains 100 curated ingredients: the original 15 unchanged identities and 85 international additions. The 19 strata describe illustrative usage contexts, not exclusive origins. This is a purposive stress test, not a random sample or an estimate of all ingredients worldwide. Strata contain only 2–15 items; their percentages should not be generalized to entire cuisines.

## Match rules

- **E**: a usable culinary ingredient concept preserving the identity and required form; source labels can need editorial cleanup. A compatible raw/dried presentation may qualify. This is not an automatically approved exact ontology mapping or an assertion that the entry is ready to import unchanged.
- **R**: a reviewed related concept, such as a plant instead of an edible part, a broader condiment family, or a different processing form. It is not counted as usable coverage.
- **N**: no specific match confirmed in the FoodOn snapshot after label/synonym and fallback review. This is not proof of absence from every release or every possible synonym.
- **BLOCKED**: not assessed because source access is unavailable. It must never be interpreted as no match.

Wikidata results are observed lower bounds from indexed search plus direct public-item, redirect, Wikipedia sitelink and plant-to-food relation review. The API was unavailable, so the related-only rows can still hide more specific items. Public-page representations are cached at different times, not one atomic Wikidata dump. A name hit, article title, disambiguation page, or taxon alone does not establish an ingredient match.

FoodOn was assessed from the full retained OWL snapshot, including labels, synonyms, definitions and food classifications. FOODON-prefixed EFSA FoodEx2 food categories count where their definitions identify the food. Organism-only, wrong-part and overly broad matches do not. The source commit and checksum are recorded in `source-snapshots.json`.

The same material presented whole or ground can qualify where this does not change the intended culinary identity: for example, the sumac fruit spice. Required transformations remain significant: smoked paprika, fermented pastes, leaves instead of seeds, and dry masa harina instead of hydrated masa dough need the appropriate concept. These judgments measure usable discovery coverage, not formal exact equivalence. Because Wikidata used public-page discovery while FoodOn used a full snapshot, this is a comparison of observed usable matches rather than a definitive ranking of source completeness.

## Observed coverage

| Source | Usable E | Related R | No specific match confirmed N | Not assessed |
|---|---:|---:|---:|---:|
| Wikidata | 68/100 | 32/100 | 0 | 0 |
| FoodOn | 50/100 | 30/100 | 20/100 | 0 |
| USDA FoodData Central | — | — | — | 100/100 |

The two reviewed sources supply a usable concept for **83/100** benchmark ingredients. This is a candidate-discovery union, not a validated merged seed: identity collisions and source-specific licensing still need review.

## Coverage by sampling stratum

| Stratum | Items | Wikidata usable | FoodOn usable | Either usable |
|---|---:|---:|---:|---:|
| Existing seed controls | 15 | 12 | 13 | 14 |
| Korean | 5 | 4 | 0 | 4 |
| Japanese | 5 | 4 | 1 | 4 |
| Chinese and Taiwanese | 5 | 4 | 1 | 4 |
| Mainland Southeast Asian | 5 | 2 | 4 | 5 |
| Maritime Southeast Asian | 5 | 3 | 2 | 3 |
| Filipino | 5 | 2 | 3 | 4 |
| South Asian | 5 | 2 | 2 | 3 |
| Iranian Central Asian and Caucasian | 5 | 3 | 0 | 3 |
| Levantine and Anatolian | 5 | 5 | 2 | 5 |
| North African | 5 | 4 | 2 | 4 |
| Horn of Africa | 5 | 4 | 1 | 4 |
| West African | 5 | 4 | 3 | 5 |
| Eastern and Southern African | 5 | 0 | 4 | 4 |
| Mexican and Central American | 5 | 2 | 3 | 4 |
| Andean and Amazonian | 5 | 4 | 1 | 4 |
| European | 5 | 5 | 5 | 5 |
| Caribbean | 3 | 2 | 1 | 2 |
| Pacific | 2 | 2 | 2 | 2 |

## Findings

Wikidata contributes named fermented foods and seasoning blends: gochujang, doenjang, saeujeot, cheonggukjang, shio koji, doubanjiang, douchi, padaek, amchur, pomegranate molasses, berbere, mitmita, niter kibbeh, ogiri, chuno and tucupi. Several records have sparse descriptions or assertions without references; coverage does not imply editorial completeness.

FoodOn contributes specific physical ingredient forms that the observed Wikidata lookup missed: garlic paste, black sesame seed, greater galangal rhizome, lemongrass stalk, culantro leaves, annatto seed, ajowan fruit, baobab powder, amaranth leaf, moringa leaf and Bambara groundnut seed. Some require replacing technical FoodEx2 labels with curated culinary wording.

Do not merge belacan and bagoong alamang solely because both appear as synonyms of generic shrimp paste. Do not accept the scotch bonnet mushroom as a pepper, pimento sweet pepper as allspice, Korean chili pepper as processed gochugaru, or masa dough as unambiguously dry masa harina. Resolve Wikidata redirects and distinguish tomato plant Q23501 from edible tomato Q20638126. Iru and dawadawa overlap as regional names, so the benchmark uses distinct oilseed condiment ogiri rather than counting both as separate identities.

The 17 current union gaps are reviewed cases for curation or deeper source-specific queries: smoked paprika, gochugaru, white miso, Chinese black vinegar, belacan, torch ginger flower, bagoong alamang, kokum, goraka, blue fenugreek, kashk, bsissa, korarima, mopane worm, masa harina, huacatay, ackee. Related records can help identify them, but do not preserve the full intended identity.

## Implications for Noms

Keep the curated canonical seed as the authority. Use Wikidata first to discover regional names, aliases and named preparations, and FoodOn as a complementary source of ingredient material and part/form distinctions. This benchmark does not justify importing either vocabulary wholesale: scientific categories, ambiguous aliases and regionally distinct preparations still need editorial decisions. Keep external IDs as reviewed source mappings, not canonical Noms identities. USDA's suitability and incremental coverage remain undecided until its records have been assessed.

## Remaining USDA work

Download the official FoodData Central export at https://fdc.nal.usda.gov/download-datasets/ and retain release date and SHA-256. Read ZIP members as streams, rather than extracting all nutrition tables. Assess Foundation, SR Legacy and FNDDS separately from Branded foods. A pure branded ingredient can establish product availability, but a meal, seasoning blend, or ingredient-list mention must not count as an exact ingredient record. Report generic usable coverage and additional branded-only coverage separately, with FDC IDs and descriptions for every positive result.

Direct requests to `www.wikidata.org` and `fdc.nal.usda.gov` received proxy 403 denials. The requested reviewed allowlist additions are `www.wikidata.org`, `query.wikidata.org`, `fdc.nal.usda.gov`, and `api.nal.usda.gov`. Public-page browsing supported the Wikidata review, but the browsing tool cannot materialize the USDA ZIP or access its API here. No third-party nutrition mirror was substituted for the requested source.

## Validation and regeneration

Run `python3 data/vocabulary/coverage/summarize.py` from the repository root to validate alignment and regenerate this report and `coverage.csv` from the reviewed JSON. The script does not query external sources or discover new matches.

The original 15 entries were compared with the retained pre-expansion seed and are unchanged as JSON objects. All 100 IDs, aliases and ingredient scopes align between the seed and benchmark. The six server vocabulary tests passed with the expanded seed, including incremental additions, atomic rollback, no-op reruns and popularity preservation. The research does not apply the expanded seed to a running application database.

## Ingredient matrix

The CSV includes scope, every selected source ID/URL, status and rationale. The JSON reviews retain the same records. No external mapping has been automatically inserted into the curated seed.

| # | Ingredient | Stratum | Wikidata | FoodOn | USDA |
|---:|---|---|---|---|---|
| 1 | chickpea | Existing seed controls | [E](https://www.wikidata.org/wiki/Q21156930) | [E](http://purl.obolibrary.org/obo/FOODON_03306811) | BLOCKED |
| 2 | gochujang | Existing seed controls | [E](https://www.wikidata.org/wiki/Q699637) | N | BLOCKED |
| 3 | garlic | Existing seed controls | [E](https://www.wikidata.org/wiki/Q21546392) | [E](http://purl.obolibrary.org/obo/FOODON_00003582) | BLOCKED |
| 4 | garlic powder | Existing seed controls | [E](https://www.wikidata.org/wiki/Q10716334) | [E](http://purl.obolibrary.org/obo/FOODON_03301844) | BLOCKED |
| 5 | garlic paste | Existing seed controls | [R](https://www.wikidata.org/wiki/Q21546392) | [E](http://purl.obolibrary.org/obo/FOODON_03305217) | BLOCKED |
| 6 | chili pepper | Existing seed controls | [E](https://www.wikidata.org/wiki/Q165199) | [E](http://purl.obolibrary.org/obo/FOODON_00003744) | BLOCKED |
| 7 | tomato | Existing seed controls | [E](https://www.wikidata.org/wiki/Q20638126) | [E](http://purl.obolibrary.org/obo/FOODON_03301453) | BLOCKED |
| 8 | olive oil | Existing seed controls | [E](https://www.wikidata.org/wiki/Q93165) | [E](http://purl.obolibrary.org/obo/FOODON_03301826) | BLOCKED |
| 9 | soy sauce | Existing seed controls | [E](https://www.wikidata.org/wiki/Q229385) | [E](http://purl.obolibrary.org/obo/FOODON_03544111) | BLOCKED |
| 10 | salt | Existing seed controls | [E](https://www.wikidata.org/wiki/Q11254) | [E](http://purl.obolibrary.org/obo/FOODON_00002250) | BLOCKED |
| 11 | tahini | Existing seed controls | [E](https://www.wikidata.org/wiki/Q383389) | [E](http://purl.obolibrary.org/obo/FOODON_00003855) | BLOCKED |
| 12 | sumac | Existing seed controls | [E](https://www.wikidata.org/wiki/Q16131953) | [E](http://purl.obolibrary.org/obo/FOODON_03541395) | BLOCKED |
| 13 | rice vinegar | Existing seed controls | [E](https://www.wikidata.org/wiki/Q1849012) | [E](http://purl.obolibrary.org/obo/FOODON_03307370) | BLOCKED |
| 14 | smoked paprika | Existing seed controls | [R](https://www.wikidata.org/wiki/Q3127593) | [R](http://purl.obolibrary.org/obo/FOODON_03301223) | BLOCKED |
| 15 | black sesame seeds | Existing seed controls | [R](https://www.wikidata.org/wiki/Q12000036) | [E](http://purl.obolibrary.org/obo/FOODON_00004782) | BLOCKED |
| 16 | doenjang | Korean | [E](https://www.wikidata.org/wiki/Q1196289) | [R](http://purl.obolibrary.org/obo/FOODON_03310005) | BLOCKED |
| 17 | gochugaru | Korean | [R](https://www.wikidata.org/wiki/Q48189338) | [R](http://purl.obolibrary.org/obo/FOODON_03302030) | BLOCKED |
| 18 | perilla leaf | Korean | [E](https://www.wikidata.org/wiki/Q27650144) | [R](http://purl.obolibrary.org/obo/FOODON_03412919) | BLOCKED |
| 19 | saeujeot | Korean | [E](https://www.wikidata.org/wiki/Q843252) | [R](http://purl.obolibrary.org/obo/FOODON_03309823) | BLOCKED |
| 20 | cheonggukjang | Korean | [E](https://www.wikidata.org/wiki/Q1071614) | [R](http://purl.obolibrary.org/obo/FOODON_00001078) | BLOCKED |
| 21 | white miso | Japanese | [R](https://www.wikidata.org/wiki/Q235169) | [R](http://purl.obolibrary.org/obo/FOODON_00003208) | BLOCKED |
| 22 | kombu | Japanese | [E](https://www.wikidata.org/wiki/Q11261547) | [E](http://purl.obolibrary.org/obo/FOODON_03317000) | BLOCKED |
| 23 | katsuobushi | Japanese | [E](https://www.wikidata.org/wiki/Q113739) | [R](http://purl.obolibrary.org/obo/FOODON_03311594) | BLOCKED |
| 24 | shio koji | Japanese | [E](https://www.wikidata.org/wiki/Q11428977) | [R](http://purl.obolibrary.org/obo/FOODON_03311656) | BLOCKED |
| 25 | umeboshi | Japanese | [E](https://www.wikidata.org/wiki/Q1045417) | [R](http://purl.obolibrary.org/obo/FOODON_03541493) | BLOCKED |
| 26 | doubanjiang | Chinese and Taiwanese | [E](https://www.wikidata.org/wiki/Q3273096) | [R](http://purl.obolibrary.org/obo/FOODON_03310005) | BLOCKED |
| 27 | douchi | Chinese and Taiwanese | [E](https://www.wikidata.org/wiki/Q3242685) | [R](http://purl.obolibrary.org/obo/FOODON_00001078) | BLOCKED |
| 28 | Shaoxing wine | Chinese and Taiwanese | [E](https://www.wikidata.org/wiki/Q175245) | [R](http://purl.obolibrary.org/obo/FOODON_03307312) | BLOCKED |
| 29 | Sichuan pepper | Chinese and Taiwanese | [E](https://www.wikidata.org/wiki/Q756800) | [E](http://purl.obolibrary.org/obo/FOODON_00003470) | BLOCKED |
| 30 | Chinese black vinegar | Chinese and Taiwanese | [R](https://www.wikidata.org/wiki/Q4922497) | [R](http://purl.obolibrary.org/obo/FOODON_03307370) | BLOCKED |
| 31 | galangal | Mainland Southeast Asian | [R](https://www.wikidata.org/wiki/Q734727) | [E](http://purl.obolibrary.org/obo/FOODON_03541357) | BLOCKED |
| 32 | makrut lime leaf | Mainland Southeast Asian | [E](https://www.wikidata.org/wiki/Q67202802) | [E](http://purl.obolibrary.org/obo/FOODON_03540903) | BLOCKED |
| 33 | lemongrass | Mainland Southeast Asian | [R](https://www.wikidata.org/wiki/Q33913) | [E](http://purl.obolibrary.org/obo/FOODON_00003340) | BLOCKED |
| 34 | culantro | Mainland Southeast Asian | [R](https://www.wikidata.org/wiki/Q1146782) | [E](http://purl.obolibrary.org/obo/FOODON_03540838) | BLOCKED |
| 35 | padaek | Mainland Southeast Asian | [E](https://www.wikidata.org/wiki/Q6853367) | [R](http://purl.obolibrary.org/obo/FOODON_03542615) | BLOCKED |
| 36 | belacan | Maritime Southeast Asian | [R](https://www.wikidata.org/wiki/Q1127705) | [R](http://purl.obolibrary.org/obo/FOODON_03309823) | BLOCKED |
| 37 | candlenut | Maritime Southeast Asian | [E](https://www.wikidata.org/wiki/Q104821775) | [E](http://purl.obolibrary.org/obo/FOODON_03541316) | BLOCKED |
| 38 | tempeh | Maritime Southeast Asian | [E](https://www.wikidata.org/wiki/Q120096) | [E](http://purl.obolibrary.org/obo/FOODON_03304999) | BLOCKED |
| 39 | buah keluak | Maritime Southeast Asian | [E](https://www.wikidata.org/wiki/Q12512536) | [R](http://purl.obolibrary.org/obo/FOODON_00004311) | BLOCKED |
| 40 | torch ginger flower | Maritime Southeast Asian | [R](https://www.wikidata.org/wiki/Q3442435) | N | BLOCKED |
| 41 | bagoong alamang | Filipino | [R](https://www.wikidata.org/wiki/Q1127705) | [R](http://purl.obolibrary.org/obo/FOODON_03309823) | BLOCKED |
| 42 | calamansi | Filipino | [R](https://www.wikidata.org/wiki/Q849387) | [E](http://purl.obolibrary.org/obo/FOODON_03541458) | BLOCKED |
| 43 | annatto seed | Filipino | [R](https://www.wikidata.org/wiki/Q425902) | [E](http://purl.obolibrary.org/obo/FOODON_03000015) | BLOCKED |
| 44 | banana ketchup | Filipino | [E](https://www.wikidata.org/wiki/Q3907811) | N | BLOCKED |
| 45 | pili nut | Filipino | [E](https://www.wikidata.org/wiki/Q104188819) | [E](http://purl.obolibrary.org/obo/FOODON_00003722) | BLOCKED |
| 46 | asafoetida | South Asian | [E](https://www.wikidata.org/wiki/Q16530385) | [E](http://purl.obolibrary.org/obo/FOODON_00002936) | BLOCKED |
| 47 | ajwain | South Asian | [R](https://www.wikidata.org/wiki/Q413254) | [E](http://purl.obolibrary.org/obo/FOODON_03541377) | BLOCKED |
| 48 | amchur | South Asian | [E](https://www.wikidata.org/wiki/Q2015681) | N | BLOCKED |
| 49 | kokum | South Asian | [R](https://www.wikidata.org/wiki/Q2095514) | [R](http://purl.obolibrary.org/obo/FOODON_03412803) | BLOCKED |
| 50 | goraka | South Asian | [R](https://www.wikidata.org/wiki/Q18187249) | [R](http://purl.obolibrary.org/obo/FOODON_03412780) | BLOCKED |
| 51 | dried lime | Iranian Central Asian and Caucasian | [E](https://www.wikidata.org/wiki/Q460570) | [R](http://purl.obolibrary.org/obo/FOODON_00003661) | BLOCKED |
| 52 | barberry | Iranian Central Asian and Caucasian | [E](https://www.wikidata.org/wiki/Q139555604) | [R](http://purl.obolibrary.org/obo/FOODON_03412492) | BLOCKED |
| 53 | blue fenugreek | Iranian Central Asian and Caucasian | [R](https://www.wikidata.org/wiki/Q164849) | [R](http://purl.obolibrary.org/obo/FOODON_03415082) | BLOCKED |
| 54 | tklapi | Iranian Central Asian and Caucasian | [E](https://www.wikidata.org/wiki/Q2906012) | N | BLOCKED |
| 55 | kashk | Iranian Central Asian and Caucasian | [R](https://www.wikidata.org/wiki/Q1113301) | N | BLOCKED |
| 56 | bulgur | Levantine and Anatolian | [E](https://www.wikidata.org/wiki/Q250432) | [E](http://purl.obolibrary.org/obo/FOODON_00003143) | BLOCKED |
| 57 | freekeh | Levantine and Anatolian | [E](https://www.wikidata.org/wiki/Q771079) | [E](http://purl.obolibrary.org/obo/FOODON_00004577) | BLOCKED |
| 58 | pomegranate molasses | Levantine and Anatolian | [E](https://www.wikidata.org/wiki/Q108946643) | [R](http://purl.obolibrary.org/obo/FOODON_03543248) | BLOCKED |
| 59 | mahlab | Levantine and Anatolian | [E](https://www.wikidata.org/wiki/Q465554) | N | BLOCKED |
| 60 | Aleppo pepper | Levantine and Anatolian | [E](https://www.wikidata.org/wiki/Q486606) | [R](http://purl.obolibrary.org/obo/FOODON_00003744) | BLOCKED |
| 61 | harissa | North African | [E](https://www.wikidata.org/wiki/Q966327) | [E](http://purl.obolibrary.org/obo/FOODON_00002484) | BLOCKED |
| 62 | preserved lemon | North African | [E](https://www.wikidata.org/wiki/Q2723408) | [E](http://purl.obolibrary.org/obo/FOODON_03316981) | BLOCKED |
| 63 | ras el hanout | North African | [E](https://www.wikidata.org/wiki/Q1067261) | N | BLOCKED |
| 64 | smen | North African | [E](https://www.wikidata.org/wiki/Q2397413) | N | BLOCKED |
| 65 | bsissa | North African | [R](https://www.wikidata.org/wiki/Q2527136) | N | BLOCKED |
| 66 | teff | Horn of Africa | [E](https://www.wikidata.org/wiki/Q103205493) | [E](http://purl.obolibrary.org/obo/FOODON_03000255) | BLOCKED |
| 67 | berbere | Horn of Africa | [E](https://www.wikidata.org/wiki/Q818763) | N | BLOCKED |
| 68 | mitmita | Horn of Africa | [E](https://www.wikidata.org/wiki/Q1762278) | N | BLOCKED |
| 69 | niter kibbeh | Horn of Africa | [E](https://www.wikidata.org/wiki/Q2697883) | N | BLOCKED |
| 70 | korarima | Horn of Africa | [R](https://www.wikidata.org/wiki/Q4689432) | N | BLOCKED |
| 71 | egusi | West African | [E](https://www.wikidata.org/wiki/Q10771593) | [E](http://purl.obolibrary.org/obo/FOODON_00003899) | BLOCKED |
| 72 | iru | West African | [E](https://www.wikidata.org/wiki/Q137835655) | N | BLOCKED |
| 73 | ogiri | West African | [E](https://www.wikidata.org/wiki/Q7080074) | N | BLOCKED |
| 74 | grains of paradise | West African | [E](https://www.wikidata.org/wiki/Q84574093) | [E](http://purl.obolibrary.org/obo/FOODON_03305445) | BLOCKED |
| 75 | fonio | West African | [R](https://www.wikidata.org/wiki/Q12439809) | [E](http://purl.obolibrary.org/obo/FOODON_03540027) | BLOCKED |
| 76 | baobab fruit powder | Eastern and Southern African | [R](https://www.wikidata.org/wiki/Q3360594) | [E](http://purl.obolibrary.org/obo/FOODON_00005549) | BLOCKED |
| 77 | mopane worm | Eastern and Southern African | [R](https://www.wikidata.org/wiki/Q311788) | N | BLOCKED |
| 78 | amaranth leaf | Eastern and Southern African | [R](https://www.wikidata.org/wiki/Q156344) | [E](http://purl.obolibrary.org/obo/FOODON_03000026) | BLOCKED |
| 79 | moringa leaf | Eastern and Southern African | [R](https://www.wikidata.org/wiki/Q234193) | [E](http://purl.obolibrary.org/obo/FOODON_00005327) | BLOCKED |
| 80 | Bambara groundnut | Eastern and Southern African | [R](https://www.wikidata.org/wiki/Q338219) | [E](http://purl.obolibrary.org/obo/FOODON_00002775) | BLOCKED |
| 81 | masa harina | Mexican and Central American | [R](https://www.wikidata.org/wiki/Q1006089) | [R](http://purl.obolibrary.org/obo/FOODON_00002529) | BLOCKED |
| 82 | epazote | Mexican and Central American | [R](https://www.wikidata.org/wiki/Q640032) | [E](http://purl.obolibrary.org/obo/FOODON_03540909) | BLOCKED |
| 83 | huitlacoche | Mexican and Central American | [R](https://www.wikidata.org/wiki/Q1430040) | [E](http://purl.obolibrary.org/obo/FOODON_03540785) | BLOCKED |
| 84 | achiote paste | Mexican and Central American | [E](https://www.wikidata.org/wiki/Q7302262) | N | BLOCKED |
| 85 | chipotle pepper | Mexican and Central American | [E](https://www.wikidata.org/wiki/Q860808) | [E](http://purl.obolibrary.org/obo/FOODON_00002563) | BLOCKED |
| 86 | aji amarillo | Andean and Amazonian | [E](https://www.wikidata.org/wiki/Q61413777) | [R](http://purl.obolibrary.org/obo/FOODON_03540653) | BLOCKED |
| 87 | huacatay | Andean and Amazonian | [R](https://www.wikidata.org/wiki/Q599870) | [R](http://purl.obolibrary.org/obo/FOODON_03414999) | BLOCKED |
| 88 | chuño | Andean and Amazonian | [E](https://www.wikidata.org/wiki/Q2352129) | N | BLOCKED |
| 89 | rocoto pepper | Andean and Amazonian | [E](https://www.wikidata.org/wiki/Q104006555) | [E](http://purl.obolibrary.org/obo/FOODON_03540656) | BLOCKED |
| 90 | tucupi | Andean and Amazonian | [E](https://www.wikidata.org/wiki/Q491222) | N | BLOCKED |
| 91 | saffron | European | [E](https://www.wikidata.org/wiki/Q25434) | [E](http://purl.obolibrary.org/obo/FOODON_00003441) | BLOCKED |
| 92 | caper | European | [E](https://www.wikidata.org/wiki/Q3229775) | [E](http://purl.obolibrary.org/obo/FOODON_00003493) | BLOCKED |
| 93 | caraway seed | European | [E](https://www.wikidata.org/wiki/Q21155816) | [E](http://purl.obolibrary.org/obo/FOODON_00003023) | BLOCKED |
| 94 | rye flour | European | [E](https://www.wikidata.org/wiki/Q10658708) | [E](http://purl.obolibrary.org/obo/FOODON_03302492) | BLOCKED |
| 95 | lingonberry | European | [E](https://www.wikidata.org/wiki/Q12339956) | [E](http://purl.obolibrary.org/obo/FOODON_00003542) | BLOCKED |
| 96 | ackee | Caribbean | [R](https://www.wikidata.org/wiki/Q102828166) | [R](http://purl.obolibrary.org/obo/FOODON_00003489) | BLOCKED |
| 97 | allspice | Caribbean | [E](https://www.wikidata.org/wiki/Q1963621) | [E](http://purl.obolibrary.org/obo/FOODON_00002667) | BLOCKED |
| 98 | Scotch bonnet pepper | Caribbean | [E](https://www.wikidata.org/wiki/Q466854) | [R](http://purl.obolibrary.org/obo/FOODON_03412555) | BLOCKED |
| 99 | taro | Pacific | [E](https://www.wikidata.org/wiki/Q134452914) | [E](http://purl.obolibrary.org/obo/FOODON_00003752) | BLOCKED |
| 100 | coconut milk | Pacific | [E](https://www.wikidata.org/wiki/Q841779) | [E](http://purl.obolibrary.org/obo/FOODON_03303084) | BLOCKED |
