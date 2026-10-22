#!/usr/bin/env python3
"""Validate the hand-reviewed matrix and regenerate its CSV and Markdown report."""
import collections
import csv
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def require(condition, message):
    if not condition:
        raise ValueError(message)


def main():
    benchmark = json.loads((ROOT / 'benchmark.json').read_text())['ingredients']
    maintained = json.loads((ROOT.parent / 'ingredients.json').read_text())['entries']
    by_id = {entry['id']: entry for entry in maintained}
    require(len(by_id) == len(maintained), 'Duplicate maintained seed IDs')
    require(all(row['id'] in by_id for row in benchmark), 'Benchmark entry missing from maintained seed')
    # Coverage remains a fixed 100-ingredient benchmark as the maintained catalog grows.
    seed = [by_id[row['id']] for row in benchmark]
    reviews = {s: json.loads((ROOT / f'{s}-review.json').read_text())
               for s in ('wikidata', 'foodon')}
    usda = json.loads((ROOT / 'usda-review.json').read_text())
    snapshots = json.loads((ROOT / 'source-snapshots.json').read_text())['usda']['archives']
    by_snapshot = {s['key']: s for s in snapshots}
    require(len(benchmark) == len(seed) == 100, 'Expected 100 benchmark and seed entries')
    require(len({r['id'] for r in benchmark}) == 100, 'Duplicate benchmark IDs')
    require([(r['id'], r['label'], r['aliases']) for r in benchmark] ==
            [(r['id'], r['label'], r['aliases']) for r in seed], 'Benchmark/seed identities differ')
    for source, rows in {**reviews, 'usda': usda}.items():
        require([r['number'] for r in rows] == list(range(1, 101)), f'{source}: row numbers')
        require([r['ingredient'] for r in rows] == [b['label'] for b in benchmark], f'{source}: labels')
        allowed = {'E', 'B', 'R', 'N'} if source == 'usda' else {'E', 'R', 'N'}
        require(all(r['status'] in allowed for r in rows), f'{source}: invalid status')
    for row, ingredient in zip(usda, benchmark):
        require(row['canonical_id'] == ingredient['id'], 'USDA canonical identity differs')
        gs, bs = row['generic_status'], row['branded_status']
        require(gs in {'E', 'R', 'N'} and bs in {'E', 'R', 'N', 'NOT_ASSESSED'}, 'USDA group status')
        require((bs == 'NOT_ASSESSED') == (gs == 'E'), 'Branded pass must target generic gaps')
        expected = ('E' if gs == 'E' else 'B' if bs == 'E' else
                    'R' if 'R' in (gs, bs) else 'N')
        require(row['status'] == expected, 'Inconsistent USDA combined status')
        for group in ('generic', 'branded'):
            status, records = row[group + '_status'], row[group + '_records']
            require(bool(records) == (status in {'E', 'R'}), 'USDA evidence/status mismatch')
            require(bool(row[group + '_reason']) and bool(row['queries'][group]), 'Missing USDA rationale/queries')
            require(len({r['source_id'] for r in records}) == len(records), 'Duplicate USDA evidence')
            for record in records:
                snapshot = by_snapshot[record['snapshot']]
                fd = record['source_id']
                require(record['status'] == status and fd.isdigit(), 'USDA record status or ID')
                require(record['data_type'] == snapshot['reviewed_data_type'], 'USDA record data type')
                require(record['release'] == snapshot['release'], 'USDA record release')
                require(record['url'] == f'https://fdc.nal.usda.gov/food-details/{fd}/nutrients', 'USDA record URL')
                require(bool(record['description']) and bool(record['reason']), 'USDA record missing evidence')
                require((record['snapshot'] == 'branded') == (group == 'branded'), 'Generic/product mix-up')
                if group == 'branded' and status == 'E':
                    require(bool(record['ingredients']), 'Product positives require ingredient evidence')
                    require(not record['discontinued_date'], 'Selected product marked discontinued')

    fields = ['number', 'ingredient', 'stratum', 'scope']
    for source in reviews:
        fields += [f'{source}_{field}' for field in ('status', 'id', 'url', 'reason')]
    fields += ['usda_status', 'usda_reason']
    for group in ('generic', 'branded'):
        fields += [f'usda_{group}_{field}' for field in ('status', 'ids', 'urls', 'reason', 'records')]
    with (ROOT / 'coverage.csv').open('w', newline='') as handle:
        writer = csv.DictWriter(handle, fieldnames=fields, lineterminator='\n')
        writer.writeheader()
        for i, ingredient in enumerate(benchmark):
            row = dict(number=i + 1, ingredient=ingredient['label'],
                       stratum=ingredient['stratum'], scope=ingredient['scope'])
            for source, rows in reviews.items():
                match = rows[i]
                row.update({f'{source}_status': match['status'], f'{source}_id': match['source_id'],
                            f'{source}_url': match['url'], f'{source}_reason': match['reason']})
            u = usda[i]
            row.update(usda_status=u['status'], usda_reason=u['generic_reason'] + ' ' + u['branded_reason'])
            for group in ('generic', 'branded'):
                records = u[group + '_records']
                row.update({f'usda_{group}_status': u[group + '_status'],
                            f'usda_{group}_ids': ' | '.join(r['source_id'] for r in records),
                            f'usda_{group}_urls': ' | '.join(r['url'] for r in records),
                            f'usda_{group}_reason': u[group + '_reason'],
                            f'usda_{group}_records': json.dumps(records, ensure_ascii=False, separators=(',', ':'))})
            writer.writerow(row)

    counts = {s: collections.Counter(r['status'] for r in rows) for s, rows in reviews.items()}
    generic = collections.Counter(r['generic_status'] for r in usda)
    branded = collections.Counter(r['branded_status'] for r in usda)
    old = [any(rows[i]['status'] == 'E' for rows in reviews.values()) for i in range(100)]
    with_generic = [old[i] or usda[i]['generic_status'] == 'E' for i in range(100)]
    with_products = [with_generic[i] or usda[i]['branded_status'] == 'E' for i in range(100)]
    generic_additions = [b['label'] for i, b in enumerate(benchmark) if with_generic[i] and not old[i]]
    product_additions = [b['label'] for i, b in enumerate(benchmark) if with_products[i] and not with_generic[i]]
    gaps = [b['label'] for i, b in enumerate(benchmark) if not with_products[i]]
    types = {s['reviewed_data_type']: sum(any(r['status'] == 'E' and r['snapshot'] == s['key']
                  for r in row['generic_records']) for row in usda) for s in snapshots if s['key'] != 'branded'}
    groups = list(dict.fromkeys(b['stratum'] for b in benchmark))
    lines = ['# International ingredient coverage review', '',
        'Status: USDA review completed 2026-10-22 using the official bulk releases below. Wikidata and FoodOn judgments remain the 2026-10-21 review; they were not rediscovered in this pass.', '',
        f"**USDA supplies usable generic records for {generic['E']}/100 ingredients, plus {branded['E']} additional branded-only ingredient products: {generic['E'] + branded['E']}/100 with either kind of USDA evidence.** The Wikidata/FoodOn union grows from {sum(old)} to {sum(with_generic)} using generic USDA records, or {sum(with_products)} when branded-only product evidence is also allowed. These are observed discovery matches, not approved source mappings.", '',
        'The benchmark retains the original 15 seed identities and adds 85 international ingredients across 19 illustrative usage strata. This is a purposive long-tail stress test, not a random or frequency-weighted estimate of all ingredients worldwide. Strata contain only 2–15 items and do not represent entire cuisines or exclusive origins.', '',
        '## Match rules', '',
        '- **E**: usable ingredient concept or generic food record preserving the scoped identity, part and required form. Compatible raw/dried or whole/ground presentations may qualify when identity is established. This is not formal exact ontology equivalence or permission to import unchanged.',
        '- **B**: usable USDA branded-only ingredient product; no generic E was confirmed. This is product evidence, kept separate from generic concept coverage.',
        '- **R**: a reviewed broader, ambiguous or differently processed food/material/product; excluded from usable totals.',
        '- **N**: no scoped match confirmed by the saved searches and review in the specified release. This is not proof of absence under every possible synonym or future release.',
        '- **—** in the additional-product column: not hand-graded because generic USDA coverage already exists. Branded coverage for all 100 ingredients is not asserted.', '',
        'Normal formulation ingredients are permitted for a benchmark that is itself a named preparation, such as gochujang, harissa or ras el hanout. A meal, unrelated seasoning blend, flavored product, or ingredient-list mention alone does not count for a constituent ingredient. Required transformations and material distinctions remain significant: smoked paprika, dry masa harina, leaf versus seed, garlic versus garlic paste, and regionally distinct fermented shrimp preparations.', '',
        '## Method and pinned evidence', '',
        'USDA archives were downloaded from the official download page on 2026-10-21 over the inherited proxy with TLS verification. The updated runtime reports unrestricted, enforced HTTP policy. The bulk download blocker is resolved; API access was not reverified and no personal API key or DEMO_KEY search quota was needed. Archive URLs, exact byte lengths, SHA-256 values and ZIP member identities are in `source-snapshots.json`.', '',
        '| USDA data type | Release | Eligible food records | Observed usable benchmark matches |',
        '|---|---|---:|---:|']
    names = {'foundation': 'Foundation', 'sr_legacy': 'SR Legacy', 'fndds': 'FNDDS', 'branded': 'Branded'}
    for snapshot in snapshots:
        usable = str(types[snapshot['reviewed_data_type']]) + '/100' if snapshot['key'] != 'branded' else f"{branded['E']} additional / {100-generic['E']} generic gaps"
        lines.append(f"| {names[snapshot['key']]} | [{snapshot['release']}]({snapshot['url']}) | {snapshot['eligible_food_rows']:,} | {usable} |")
    lines += ['',
        'Each complete `food.csv` was streamed without extracting nutrition tables. Only `foundation_food`, `sr_legacy_food`, `survey_fndds_food` and `branded_food` rows are eligible; Foundation sample and acquisition rows are excluded from generic coverage. Description searches use saved labels, aliases and fallback phrases with case/diacritic folding and word boundaries. Generic fallbacks include USDA naming variants such as pilinuts and drumstick leaves. Searches produce candidates; identity/part/form judgments are manual.', '',
        f"For the {100-generic['E']} generic gaps, branded candidate descriptions were joined to `branded_food.csv` for ingredient and product checks. Ingredient-list-only mentions were not used as independent ingredient records or as a reverse ingredient-list search. `usda-review.json` retains all queries, judgments, selected FDC IDs, exact descriptions, data types, releases, URLs, reasons and selected product ingredients. It retains representative evidence, not every duplicate nutritional/product version. The targeted branded pass supports an incremental count, not a standalone branded-completeness percentage.", '',
        'Wikidata remains an observed lower bound from indexed search, public entity pages, redirects, sitelinks and plant-to-food relation traversal. Its API was unavailable in the earlier review; cached representations were not an atomic snapshot. FoodOn was reviewed from the full pinned OWL snapshot, including definitions/synonyms and specific FoodEx2 categories. Because source representations and discovery methods differ, this is an observed-coverage comparison, not a definitive completeness ranking.', '',
        '## Observed coverage', '',
        '| Source / pass | Usable | Related R | No scoped match confirmed N | Not assessed |',
        '|---|---:|---:|---:|---:|',
        f"| Wikidata | {counts['wikidata']['E']}/100 E | {counts['wikidata']['R']} | {counts['wikidata']['N']} | 0 |",
        f"| FoodOn | {counts['foodon']['E']}/100 E | {counts['foodon']['R']} | {counts['foodon']['N']} | 0 |",
        f"| USDA generic union | {generic['E']}/100 E | {generic['R']} | {generic['N']} | 0 |",
        f"| USDA additional products | {branded['E']} B | {branded['R']} | {branded['N']} | {branded['NOT_ASSESSED']} already generic-covered |", '',
        f"Foundation, SR Legacy and FNDDS overlap; their usable counts must not be added. The generic union is {generic['E']}/100. Adding the {branded['E']} branded-only products gives {generic['E']+branded['E']}/100 observed USDA coverage; {100-generic['E']-branded['E']} ingredients remain related-only or without a confirmed match.", '',
        f"USDA generic evidence fills {len(generic_additions)} Wikidata/FoodOn gap: {', '.join(generic_additions)}. Additional product evidence fills {len(product_additions)} more: {', '.join(product_additions)}. The resulting three-source discovery union is **{sum(with_generic)}/100 with generic USDA evidence**, or **{sum(with_products)}/100 including product evidence**. A product record does not turn the latter into generic ontology coverage.", '',
        '## Coverage by sampling stratum', '',
        '| Stratum | Items | Wikidata E | FoodOn E | USDA generic E | USDA additional B | Concept/generic union | Including products |',
        '|---|---:|---:|---:|---:|---:|---:|---:|']
    for group in groups:
        ix = [i for i, row in enumerate(benchmark) if row['stratum'] == group]
        values = [len(ix), sum(reviews['wikidata'][i]['status'] == 'E' for i in ix),
                  sum(reviews['foodon'][i]['status'] == 'E' for i in ix),
                  sum(usda[i]['generic_status'] == 'E' for i in ix),
                  sum(usda[i]['branded_status'] == 'E' for i in ix),
                  sum(with_generic[i] for i in ix), sum(with_products[i] for i in ix)]
        lines.append('| ' + group + ' | ' + ' | '.join(map(str, values)) + ' |')
    lines += ['', '## Findings and limits', '',
        'Wikidata contributes named regional fermented foods and seasoning blends. FoodOn contributes edible-part and processed-material distinctions, including garlic paste, black sesame, greater galangal rhizome, culantro leaves, annatto seed, baobab powder, moringa leaves and Bambara groundnut. USDA adds useful generic food forms, including explicit dry masa harina, but most of its incremental regional coverage in this benchmark comes from branded products.', '',
        'Specific USDA safeguards: unsmoked paprika and unspecified miso remain R for smoked paprika and white miso until a named product supplies the missing form. Generic fermented shrimp sauce does not establish belacan or saeujeot. Belacan is supported separately by the named BLACHEN dried shrimp paste product, and bagoong alamang has its own named product; neither is inferred solely from generic shrimp-paste synonyms. Galangal powder lacks greater-galangal species/part evidence; bonito flakes do not explicitly confirm the required smoking/fermentation. Salted sumac, huacatay paste, prepared huitlacoche, lingonberry preserves and Scotch bonnet sauces are retained as related products under the scoped ingredient rules. These conservative judgments can be revisited with stronger evidence; they are not claims that the foods are unavailable in commerce.', '',
        'Reject lexical traps: CHUNO PURE chocolate is not freeze-dried potato, li-hing snacks are not asafoetida, pimento sweet pepper is not allspice, and a Scotch bonnet mushroom is not a pepper. Do not merge belacan with bagoong merely through a generic shrimp-paste synonym, Korean chili pepper with gochugaru, or wet masa dough with dry masa harina. Resolve Wikidata plant/food distinctions and redirects before mapping canonical ingredients.', '',
        'USDA branded data are supplier-submitted product records and can be historical, duplicated or poorly labeled; a release date is not a product publication date or a guarantee of current availability. Selected positives have no supplied discontinuation date, but that does not establish present stock. The niter-kibbeh record has an unexpected brand-owner value, illustrating why these are research candidates rather than import-ready authority. Product ingredients were reviewed for identity, not audited for authenticity or nutritional interchangeability.', '',
        f"The {len(gaps)} remaining observed union gaps, even allowing product evidence, are: {', '.join(gaps)}. Related records may help deeper curation, but do not preserve the required scoped identity in this review.", '',
        '## Implications for Noms', '',
        'Keep the curated seed as the authority. Use Wikidata for regional names and preparations, FoodOn for material/part/form distinctions, and generic USDA records for food/nutrition-linked evidence. Use USDA products as supplemental discovery leads with explicit product status; they do not replace a concept vocabulary. Preserve Noms canonical IDs and review external mappings separately. No source mappings were promoted and no canonical ingredient data were changed by this research.', '',
        '## Validation and regeneration', '',
        'Run `python3 data/vocabulary/coverage/summarize.py` to validate alignment and regenerate this report and `coverage.csv`. It does not query sources or discover matches. To audit the retained USDA evidence, download the pinned archives to the `local_filename` names and run `python3 data/vocabulary/coverage/inspect_usda.py --archives DIR`. It verifies SHA-256, ZIP members, eligible record counts, exact selected descriptions/publication dates and selected product metadata/ingredients while rerunning the saved description searches. Optional `--candidates PATH` writes a generated candidate CSV outside the checkout; judgments are never automatically assigned.', '',
        'This update changes research artifacts and their analysis scripts. It does not claim a new application-wide test run. The separate local app startup loaded the existing 100-entry seed; the source investigation itself performs no database writes.', '',
        '## Ingredient matrix', '',
        'Links select representative evidence. JSON/CSV retain all selected generic data-type representatives and full product evidence. An E in the additional-product column contributes B to the USDA combined status.', '',
        '| # | Ingredient | Stratum | Wikidata | FoodOn | USDA generic | USDA additional product |',
        '|---:|---|---|---|---|---|---|']
    def cell(status, url):
        return f'[{status}]({url})' if url else status
    for i, b in enumerate(benchmark):
        u = usda[i]
        cells = [cell(reviews[s][i]['status'], reviews[s][i]['url']) for s in reviews]
        for group in ('generic', 'branded'):
            records = u[group + '_records']
            status = u[group + '_status']
            if group == 'branded' and status == 'E':
                status = 'B'
            cells.append(cell('—' if status == 'NOT_ASSESSED' else status,
                              records[0]['url'] if records else ''))
        lines.append(f"| {i+1} | {b['label']} | {b['stratum']} | " + ' | '.join(cells) + ' |')
    (ROOT / 'README.md').write_text('\n'.join(lines) + '\n')
    print(json.dumps(dict(ingredients=100, strata=len(groups),
        wikidata=dict(counts['wikidata']), foodon=dict(counts['foodon']),
        usda_generic=dict(generic), usda_additional_branded=dict(branded),
        generic_matches_by_data_type=types, original_union=sum(old),
        union_with_generic_usda=sum(with_generic), union_including_products=sum(with_products),
        generic_additions=generic_additions, product_additions=product_additions)))


if __name__ == '__main__':
    main()
