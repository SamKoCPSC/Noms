#!/usr/bin/env python3
"""Capture a bounded Wikidata equipment research snapshot; never seed a database.

Discovery keeps all type/instance IDs. Detailed capture includes classes and
English-Wikipedia-linked instances, not thousands of individual museum objects.
The immutable revision cache also supports exact offline-review restoration.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import datetime
import json
from pathlib import Path

from fetch_wikidata import ENDPOINT, query, revision_batch
from foodon import digest, load, require

ROOTS = {'Q26037047': 'culinary equipment', 'Q1521410': 'kitchenware',
         'Q3773693': 'kitchen utensil', 'Q154038': 'cookware and bakeware'}
MODES = ('classes', 'instances', 'article_instances')


def discovery_query(qid, mode):
    expression = f'?item wdt:P279* wd:{qid}' if mode == 'classes' else f'?item wdt:P31/wdt:P279* wd:{qid}'
    if mode == 'article_instances':
        expression += '. ?article schema:about ?item; schema:isPartOf <https://en.wikipedia.org/>'
    return ('SELECT DISTINCT ?item ?revision WHERE { ' + expression +
            '. ?item schema:version ?revision. '
            'FILTER(STRSTARTS(STR(?item), "http://www.wikidata.org/entity/Q")) }\n')


def capture(args):
    require(not args.output.exists(), 'Choose a fresh output directory')
    args.cache.mkdir(parents=True, exist_ok=True)
    if args.restore:
        descriptor = load(args.restore)
    else:
        args.discovery.mkdir(parents=True, exist_ok=True)
        registry, queries = {}, []
        for qid in ROOTS:
            for mode in MODES:
                text = discovery_query(qid, mode)
                path = args.discovery / f'{qid}-{mode}.json'
                if not path.exists():
                    path.write_text(json.dumps(query(text)) + '\n')
                rows = load(path)['results']['bindings']
                queries.append(dict(root=qid, mode=mode, query=text, checksum=digest(path), rows=len(rows)))
                for row in rows:
                    key = row['item']['value'].rsplit('/', 1)[-1]
                    require(key.startswith('Q') and key[1:].isdigit(), 'Non-item in discovery')
                    revision = int(row['revision']['value'])
                    record = registry.setdefault(key, dict(revision=revision, memberships=[]))
                    # Different queries are not one atomic global dump. Keep the
                    # latest observed entity revision and retain every raw result.
                    record['revision'] = max(record['revision'], revision)
                    record['memberships'].append(dict(root=qid, mode=mode))
        revisions = {qid: row['revision'] for qid, row in registry.items()
                     if any(m['mode'] in {'classes', 'article_instances'} for m in row['memberships'])}
        descriptor = dict(schema_version=1,
            captured_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),
            query_endpoint=ENDPOINT, roots=ROOTS, queries=queries,
            raw_records=len(registry), detailed_records=len(revisions),
            scope='All discovered classes plus English-Wikipedia-linked instances. Other instance records remain in discovery, not the detailed review. Not all equipment in Wikidata.',
            revisions=dict(sorted(revisions.items(), key=lambda x: int(x[0][1:]))),
            class_ids=sorted(qid for qid in revisions if any(m['mode'] == 'classes' for m in registry[qid]['memberships'])),
            article_instance_ids=sorted(qid for qid in revisions if any(m['mode'] == 'article_instances' for m in registry[qid]['memberships'])))
        (args.discovery / 'registry.json').write_text(json.dumps(registry, sort_keys=True) + '\n')
    pairs = list(descriptor['revisions'].items())
    batches = [pairs[i:i + 50] for i in range(0, len(pairs), 50)]
    entities = []
    with ThreadPoolExecutor(max_workers=2) as pool:
        for batch in pool.map(lambda rows: revision_batch(rows, args.cache), batches):
            entities.extend(batch)
            print(f'Captured {len(entities)}/{len(pairs)} pinned equipment entities', flush=True)
    args.output.mkdir(parents=True)
    path = args.output / 'entities.json'
    path.write_text(json.dumps(dict(schema_version=1, entities=sorted(entities, key=lambda e: int(e['id'][1:]))),
                               ensure_ascii=False, sort_keys=True, separators=(',', ':')) + '\n')
    if args.restore:
        require(path.stat().st_size == descriptor['bytes'] and digest(path) == descriptor['source']['checksum'],
                'Restored snapshot differs from pinned capture')
    else:
        descriptor['bytes'] = path.stat().st_size
        descriptor['source'] = dict(id='wikidata-equipment-' + descriptor['captured_at'][:10],
            url='https://www.wikidata.org/wiki/Wikidata:Data_access',
            version=descriptor['captured_at'] + ' (per-entity revisions; not an atomic dump)',
            checksum=digest(path), license='CC0-1.0',
            attribution='Wikidata contributors; structured entity data under CC0. No Wikipedia article text or images imported.')
    (args.output / 'descriptor.json').write_text(json.dumps(descriptor, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps(dict(entities=len(entities), bytes=path.stat().st_size, checksum=digest(path))))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--discovery', type=Path, required=True, help='External cache of complete query results')
    parser.add_argument('--cache', type=Path, required=True, help='External immutable entity-revision cache')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--restore', type=Path, help='Restore exact selected revisions from this descriptor')
    capture(parser.parse_args())
