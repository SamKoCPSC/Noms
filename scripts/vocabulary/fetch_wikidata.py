#!/usr/bin/env python3
"""Capture/restore pinned Wikidata revisions; offline adapter never uses the network."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import datetime
import hashlib
import json
from pathlib import Path
import time
import urllib.parse
import urllib.request
import urllib.error

from foodon import ROOT, digest, load, require

AGENT = 'NomsIngredientResearch/0.1 (https://github.com/SamKoCPSC/Noms)'
API = 'https://www.wikidata.org/w/api.php'
ENDPOINT = 'https://query.wikidata.org/sparql'
PROPERTIES = {'P31', 'P279', 'P105', 'P171', 'P366', 'P361', 'P527', 'P186', 'P460', 'P1889', 'P1056', 'P828'}


def get(url):
    for attempt in range(6):
        try:
            request = urllib.request.Request(url, headers={'User-Agent': AGENT, 'Accept': 'application/json'})
            with urllib.request.urlopen(request, timeout=60) as response:
                result = json.load(response)
            require('error' not in result, f'Wikimedia API error: {result.get("error")}')
            return result
        except (OSError, ValueError) as error:
            if attempt == 5:
                raise
            delay = min(30, 2 ** (attempt + 1))
            if isinstance(error, urllib.error.HTTPError):
                retry_after = error.headers.get('Retry-After', '')
                if retry_after.isdigit():
                    delay = max(delay, int(retry_after))
            time.sleep(delay)


def query(text):
    return get(ENDPOINT + '?' + urllib.parse.urlencode({'query': text, 'format': 'json'}))


def project(entity, revision):
    """Retain English text and semantic statements, including ranks/qualifiers/references."""
    return dict(id=entity['id'], type=entity['type'], lastrevid=revision,
                labels={k: v for k, v in (entity.get('labels') or {}).items() if k == 'en'},
                descriptions={k: v for k, v in (entity.get('descriptions') or {}).items() if k == 'en'},
                aliases={k: v for k, v in (entity.get('aliases') or {}).items() if k == 'en'},
                claims={k: v for k, v in (entity.get('claims') or {}).items() if k in PROPERTIES},
                redirect=entity.get('redirect'))


def revision_batch(pairs, cache):
    key = hashlib.sha256(json.dumps(pairs, separators=(',', ':')).encode()).hexdigest()
    path = cache / (key + '.json')
    if path.exists():
        data = load(path)
    else:
        url = API + '?' + urllib.parse.urlencode(dict(action='query', prop='revisions',
              revids='|'.join(str(rev) for _, rev in pairs), rvprop='ids|content',
              rvslots='main', format='json', formatversion=2, maxlag=10))
        data = get(url)
        path.write_text(json.dumps(data, ensure_ascii=False), encoding='utf-8')
    expected = dict(pairs)
    result = {}
    for page in data.get('query', {}).get('pages', []):
        for revision in page.get('revisions', []):
            text = revision['slots']['main']['content']
            entity = json.loads(text)
            qid, rev = entity['id'], revision['revid']
            require(expected.get(qid) == rev, f'Unexpected revision: {qid}@{rev}')
            require(qid not in result, f'Duplicate entity: {qid}')
            result[qid] = project(entity, rev)
    require(set(result) == set(expected), 'Incomplete or suppressed revision batch')
    return list(result.values())


def capture(args):
    output = args.output.resolve()
    require(not output.exists(), 'Choose a fresh output directory')
    args.cache.mkdir(parents=True, exist_ok=True)
    if args.restore:
        descriptor = load(args.restore)
        revisions = descriptor['revisions']
    else:
        text = args.query.read_text()
        discovery_path = args.cache / ('discovery-' + digest(args.query) + '.json')
        discovery = load(discovery_path) if discovery_path.exists() else query(text)
        if not discovery_path.exists():
            discovery_path.write_text(json.dumps(discovery), encoding='utf-8')
        revisions = {}
        for row in discovery['results']['bindings']:
            qid = row['item']['value'].rsplit('/', 1)[-1]
            require(qid.startswith('Q') and qid[1:].isdigit() and 'revision' in row,
                    'Discovery must return item IDs and revisions')
            rev = int(row['revision']['value'])
            require(qid not in revisions or revisions[qid] == rev, 'Conflicting discovery revisions')
            revisions[qid] = rev
        require(revisions, 'Discovery returned no item revisions')
        scope = sorted(revisions, key=lambda x: int(x[1:]))
        # Benchmark items retain their own evidence even when outside this hierarchy.
        extras = {r['source_id'] for r in load(args.benchmark) if r.get('source_id')}
        extra_path = args.cache / ('benchmark-' + digest(args.benchmark) + '.json')
        if extras:
            extra_query = 'SELECT ?item ?revision WHERE { VALUES ?item { ' + ' '.join('wd:' + q for q in sorted(extras)) + ' } ?item schema:version ?revision }'
            extra = load(extra_path) if extra_path.exists() else query(extra_query)
            extra_path.write_text(json.dumps(extra), encoding='utf-8')
            found = {r['item']['value'].rsplit('/', 1)[-1]: int(r['revision']['value']) for r in extra['results']['bindings']}
            require(set(found) == extras, 'Benchmark revision discovery incomplete')
            revisions.update({q: rev for q, rev in found.items() if q not in revisions})
        descriptor = dict(schema_version=1, captured_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),
                          query=text, query_endpoint=ENDPOINT,
                          outside_discovery_entities=sorted(set(revisions) - set(scope)),
                          revisions=dict(sorted(revisions.items(), key=lambda x: int(x[0][1:]))))
    pairs = sorted(revisions.items(), key=lambda x: int(x[0][1:]))
    batches = [pairs[i:i + 50] for i in range(0, len(pairs), 50)]
    entities = []
    with ThreadPoolExecutor(max_workers=2) as pool:
        for i, batch in enumerate(pool.map(lambda p: revision_batch(p, args.cache), batches)):
            entities.extend(batch)
            if i % 10 == 0:
                print(f'Captured {len(entities)}/{len(pairs)} pinned entities', flush=True)
    snapshot = dict(schema_version=1, entities=sorted(entities, key=lambda e: int(e['id'][1:])))
    output.mkdir(parents=True)
    path = output / 'entities.json'
    path.write_text(json.dumps(snapshot, ensure_ascii=False, sort_keys=True, separators=(',', ':')) + '\n', encoding='utf-8')
    if args.restore:
        require(path.stat().st_size == descriptor['bytes'] and digest(path) == descriptor['source']['checksum'],
                'Restored snapshot differs from pinned descriptor')
    else:
        descriptor['bytes'] = path.stat().st_size
        descriptor['source'] = dict(id='wikidata-' + descriptor['captured_at'][:10],
            url='https://www.wikidata.org/wiki/Wikidata:Data_access',
            version=descriptor['captured_at'] + ' (per-entity revisions; not an atomic global dump)',
            checksum=digest(path), license='CC0-1.0',
            attribution='Wikidata contributors; structured entity data under CC0. English text and selected ranked statements retained at the listed entity revisions; Wikipedia article text is not imported.')
    (output / 'descriptor.json').write_text(json.dumps(descriptor, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'entities': len(entities), 'bytes': path.stat().st_size, 'sha256': digest(path)}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--query', type=Path, default=ROOT / 'data/vocabulary/adapters/wikidata-ingredients.rq')
    parser.add_argument('--benchmark', type=Path, default=ROOT / 'data/vocabulary/coverage/wikidata-review.json')
    parser.add_argument('--restore', type=Path, help='Restore exact revisions from a saved descriptor')
    parser.add_argument('--cache', type=Path, required=True, help='Dedicated cache outside Git; immutable revision batches')
    parser.add_argument('--output', type=Path, required=True)
    capture(parser.parse_args())


if __name__ == '__main__':
    main()
