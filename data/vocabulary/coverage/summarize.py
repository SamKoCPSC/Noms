#!/usr/bin/env python3
"""Validate the hand-reviewed matrix and regenerate its CSV and Markdown report."""
import collections
import csv
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
benchmark = json.loads((ROOT / "benchmark.json").read_text())["ingredients"]
seed = json.loads((ROOT.parent / "ingredients.json").read_text())["entries"]
reviews = {
    source: json.loads((ROOT / f"{source}-review.json").read_text())
    for source in ("wikidata", "foodon")
}
assert len(benchmark) == len(seed) == 100
assert len({row["id"] for row in benchmark}) == 100
assert [(r["id"], r["label"]) for r in benchmark] == [(r["id"], r["label"]) for r in seed]
for source, rows in reviews.items():
    assert [r["number"] for r in rows] == list(range(1, 101)), source
    assert [r["ingredient"] for r in rows] == [r["label"] for r in benchmark], source
    assert all(r["status"] in {"E", "R", "N"} for r in rows), source

fields = ["number", "ingredient", "stratum", "scope"]
for source in reviews:
    fields += [f"{source}_{field}" for field in ["status", "id", "url", "reason"]]
fields += ["usda_status", "usda_reason"]
with (ROOT / "coverage.csv").open("w", newline="") as handle:
    writer = csv.DictWriter(handle, fieldnames=fields, lineterminator="\n")
    writer.writeheader()
    for index, ingredient in enumerate(benchmark):
        row = {"number": index + 1, "ingredient": ingredient["label"],
               "stratum": ingredient["stratum"], "scope": ingredient["scope"]}
        for source, rows in reviews.items():
            match = rows[index]
            row.update({f"{source}_status": match["status"], f"{source}_id": match["source_id"],
                        f"{source}_url": match["url"], f"{source}_reason": match["reason"]})
        row.update(usda_status="BLOCKED", usda_reason="Official bulk/API endpoints denied by current cloud network policy; not assessed, not zero coverage.")
        writer.writerow(row)

counts = {source: collections.Counter(r["status"] for r in rows) for source, rows in reviews.items()}
union = sum(any(rows[i]["status"] == "E" for rows in reviews.values()) for i in range(100))
gaps = [benchmark[i]["label"] for i in range(100)
        if not any(rows[i]["status"] == "E" for rows in reviews.values())]
groups = list(dict.fromkeys(r["stratum"] for r in benchmark))
lines = ["# International ingredient coverage review", "",
         "Status: interim. Wikidata and FoodOn have been reviewed; USDA bulk/API access is blocked by the current cloud network policy. No USDA coverage percentage is asserted.", "",
         "Reviewed 2026-10-21. The seed contains 100 curated ingredients: the original 15 unchanged identities and 85 international additions. The 19 strata describe illustrative usage contexts, not exclusive origins. This is a purposive stress test, not a random sample or an estimate of all ingredients worldwide. Strata contain only 2–15 items; their percentages should not be generalized to entire cuisines.", "",
         "## Match rules", "",
         "- **E**: a usable culinary ingredient concept preserving the identity and required form; source labels can need editorial cleanup. A compatible raw/dried presentation may qualify. This is not an automatically approved exact ontology mapping or an assertion that the entry is ready to import unchanged.",
         "- **R**: a reviewed related concept, such as a plant instead of an edible part, a broader condiment family, or a different processing form. It is not counted as usable coverage.",
         "- **N**: no specific match confirmed in the FoodOn snapshot after label/synonym and fallback review. This is not proof of absence from every release or every possible synonym.",
         "- **BLOCKED**: not assessed because source access is unavailable. It must never be interpreted as no match.", "",
         "Wikidata results are observed lower bounds from indexed search plus direct public-item, redirect, Wikipedia sitelink and plant-to-food relation review. The API was unavailable, so the related-only rows can still hide more specific items. Public-page representations are cached at different times, not one atomic Wikidata dump. A name hit, article title, disambiguation page, or taxon alone does not establish an ingredient match.", "",
         "FoodOn was assessed from the full retained OWL snapshot, including labels, synonyms, definitions and food classifications. FOODON-prefixed EFSA FoodEx2 food categories count where their definitions identify the food. Organism-only, wrong-part and overly broad matches do not. The source commit and checksum are recorded in `source-snapshots.json`.", "",
         "The same material presented whole or ground can qualify where this does not change the intended culinary identity: for example, the sumac fruit spice. Required transformations remain significant: smoked paprika, fermented pastes, leaves instead of seeds, and dry masa harina instead of hydrated masa dough need the appropriate concept. These judgments measure usable discovery coverage, not formal exact equivalence. Because Wikidata used public-page discovery while FoodOn used a full snapshot, this is a comparison of observed usable matches rather than a definitive ranking of source completeness.", "",
         "## Observed coverage", "",
         "| Source | Usable E | Related R | No specific match confirmed N | Not assessed |",
         "|---|---:|---:|---:|---:|",
         f"| Wikidata | {counts['wikidata']['E']}/100 | {counts['wikidata']['R']}/100 | 0 | 0 |",
         f"| FoodOn | {counts['foodon']['E']}/100 | {counts['foodon']['R']}/100 | {counts['foodon']['N']}/100 | 0 |",
         "| USDA FoodData Central | — | — | — | 100/100 |", "",
         f"The two reviewed sources supply a usable concept for **{union}/100** benchmark ingredients. This is a candidate-discovery union, not a validated merged seed: identity collisions and source-specific licensing still need review.", "",
         "## Coverage by sampling stratum", "",
         "| Stratum | Items | Wikidata usable | FoodOn usable | Either usable |",
         "|---|---:|---:|---:|---:|"]
for group in groups:
    indexes = [i for i, row in enumerate(benchmark) if row["stratum"] == group]
    w = sum(reviews["wikidata"][i]["status"] == "E" for i in indexes)
    f = sum(reviews["foodon"][i]["status"] == "E" for i in indexes)
    either = sum(any(rows[i]["status"] == "E" for rows in reviews.values()) for i in indexes)
    lines.append(f"| {group} | {len(indexes)} | {w} | {f} | {either} |")
lines += ["", "## Findings", "",
          "Wikidata contributes named fermented foods and seasoning blends: gochujang, doenjang, saeujeot, cheonggukjang, shio koji, doubanjiang, douchi, padaek, amchur, pomegranate molasses, berbere, mitmita, niter kibbeh, ogiri, chuno and tucupi. Several records have sparse descriptions or assertions without references; coverage does not imply editorial completeness.", "",
          "FoodOn contributes specific physical ingredient forms that the observed Wikidata lookup missed: garlic paste, black sesame seed, greater galangal rhizome, lemongrass stalk, culantro leaves, annatto seed, ajowan fruit, baobab powder, amaranth leaf, moringa leaf and Bambara groundnut seed. Some require replacing technical FoodEx2 labels with curated culinary wording.", "",
          "Do not merge belacan and bagoong alamang solely because both appear as synonyms of generic shrimp paste. Do not accept the scotch bonnet mushroom as a pepper, pimento sweet pepper as allspice, Korean chili pepper as processed gochugaru, or masa dough as unambiguously dry masa harina. Resolve Wikidata redirects and distinguish tomato plant Q23501 from edible tomato Q20638126. Iru and dawadawa overlap as regional names, so the benchmark uses distinct oilseed condiment ogiri rather than counting both as separate identities.", "",
          f"The {len(gaps)} current union gaps are reviewed cases for curation or deeper source-specific queries: " + ", ".join(gaps) + ". Related records can help identify them, but do not preserve the full intended identity.", "",
          "## Implications for Noms", "",
          "Keep the curated canonical seed as the authority. Use Wikidata first to discover regional names, aliases and named preparations, and FoodOn as a complementary source of ingredient material and part/form distinctions. This benchmark does not justify importing either vocabulary wholesale: scientific categories, ambiguous aliases and regionally distinct preparations still need editorial decisions. Keep external IDs as reviewed source mappings, not canonical Noms identities. USDA's suitability and incremental coverage remain undecided until its records have been assessed.", "",
          "## Remaining USDA work", "",
          "Download the official FoodData Central export at https://fdc.nal.usda.gov/download-datasets/ and retain release date and SHA-256. Read ZIP members as streams, rather than extracting all nutrition tables. Assess Foundation, SR Legacy and FNDDS separately from Branded foods. A pure branded ingredient can establish product availability, but a meal, seasoning blend, or ingredient-list mention must not count as an exact ingredient record. Report generic usable coverage and additional branded-only coverage separately, with FDC IDs and descriptions for every positive result.", "",
          "Direct requests to `www.wikidata.org` and `fdc.nal.usda.gov` received proxy 403 denials. The requested reviewed allowlist additions are `www.wikidata.org`, `query.wikidata.org`, `fdc.nal.usda.gov`, and `api.nal.usda.gov`. Public-page browsing supported the Wikidata review, but the browsing tool cannot materialize the USDA ZIP or access its API here. No third-party nutrition mirror was substituted for the requested source.", "",
          "## Validation and regeneration", "",
          "Run `python3 data/vocabulary/coverage/summarize.py` from the repository root to validate alignment and regenerate this report and `coverage.csv` from the reviewed JSON. The script does not query external sources or discover new matches.", "",
          "The original 15 entries were compared with the retained pre-expansion seed and are unchanged as JSON objects. All 100 IDs, aliases and ingredient scopes align between the seed and benchmark. The six server vocabulary tests passed with the expanded seed, including incremental additions, atomic rollback, no-op reruns and popularity preservation. The research does not apply the expanded seed to a running application database.", "",
          "## Ingredient matrix", "",
          "The CSV includes scope, every selected source ID/URL, status and rationale. The JSON reviews retain the same records. No external mapping has been automatically inserted into the curated seed.", "",
          "| # | Ingredient | Stratum | Wikidata | FoodOn | USDA |",
          "|---:|---|---|---|---|---|"]
for i, b in enumerate(benchmark):
    cells=[]
    for source, rows in reviews.items():
        r=rows[i]
        cells.append(f"[{r['status']}]({r['url']})" if r['url'] else r['status'])
    lines.append(f"| {i+1} | {b['label']} | {b['stratum']} | {cells[0]} | {cells[1]} | BLOCKED |")
(ROOT / "README.md").write_text("\n".join(lines)+"\n")
print(json.dumps({"ingredients":100,"strata":len(groups),"wikidata":dict(counts['wikidata']),"foodon":dict(counts['foodon']),"union_usable":union,"usda":"blocked"}))
