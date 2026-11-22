#!/usr/bin/env python3
"""Offline USDA bulk-CSV extraction and approval-bound canonical seed builds."""
import argparse
import collections
import copy
import csv
import hashlib
import io
import json
from pathlib import Path
import re
import uuid
import zipfile

from foodon import (ROOT, RELATIONSHIPS, digest, load, normalize, require,
                    valid_id, valid_text, validate, write_bundle)


def row_checksum(row):
    return hashlib.sha256(json.dumps(row, sort_keys=True, ensure_ascii=False,
                                    separators=(',', ':')).encode()).hexdigest()


def read_archives(directory, descriptor):
    require(descriptor['schema_version'] == 1, 'Unsupported USDA descriptor')
    terms, counts, source_ids, archive_keys = {}, {}, set(), set()
    for pin in descriptor['archives']:
        key, source = pin['key'], pin['source']
        require(key in {'foundation', 'sr_legacy'} and key not in archive_keys and
                source['id'] not in source_ids and source['license'] == 'CC0-1.0',
                'Invalid/repeated USDA archive or source')
        archive_keys.add(key); source_ids.add(source['id'])
        expected_type = 'foundation_food' if key == 'foundation' else 'sr_legacy_food'
        require(pin['data_type'] == expected_type and
                Path(pin['local_filename']).name == pin['local_filename'], 'Invalid archive pin')
        path = Path(directory)/pin['local_filename']
        require(path.stat().st_size == pin['bytes'] and digest(path) == source['checksum'],
                'Archive bytes/checksum differ from descriptor: ' + key)
        with zipfile.ZipFile(path) as archive:
            require(len(set(archive.namelist())) == len(archive.namelist()), 'Duplicate ZIP member')
            members = {}
            for member in pin['members']:
                info = archive.getinfo(member['name'])
                require(info.file_size == member['bytes'] and f'{info.CRC:08x}' == member['crc32'],
                        'ZIP member identity changed')
                suffix = Path(member['name']).name
                require(suffix not in members, 'Repeated input table')
                members[suffix] = member['name']

            def rows(filename):
                # Read members directly; never extract paths from an archive.
                with archive.open(members[filename]) as raw:
                    yield from csv.DictReader(io.TextIOWrapper(raw, encoding='utf-8-sig'))

            categories = {}
            for row in rows('food_category.csv'):
                require(row['id'] not in categories and valid_text(row['description']),
                        'Invalid/repeated food category')
                categories[row['id']] = row['description']
            metadata = {}
            table = 'foundation_food.csv' if key == 'foundation' else 'sr_legacy_food.csv'
            for row in rows(table):
                require(row['fdc_id'] not in metadata, 'Duplicate FDC metadata')
                metadata[row['fdc_id']] = row['NDB_number']
            seen, counter, eligible = set(), collections.Counter(), 0
            for row in rows('food.csv'):
                fdc_id = row['fdc_id']
                require(re.fullmatch(r'[1-9][0-9]*', fdc_id) and fdc_id not in seen,
                        'Invalid/repeated FDC ID')
                seen.add(fdc_id); counter[row['data_type']] += 1
                if row['data_type'] != expected_type:
                    continue  # Foundation samples/acquisitions are not ingredient records.
                require(row['food_category_id'] in categories and
                        valid_text(row['description'], 2000) and
                        re.fullmatch(r'\d{4}-\d{2}-\d{2}', row['publication_date']),
                        'Invalid food description/category/publication/metadata')
                record_id = key + ':' + fdc_id
                # The real Foundation CSV omits metadata for some food rows.
                # NDB numbers are optional cross-references, never FDC identities.
                ndb_number = metadata.get(fdc_id, '')
                evidence = dict(row, ndb_number=ndb_number)
                terms[record_id] = dict(id=record_id, fdc_id=fdc_id, archive=key,
                    source=source['id'], label=row['description'], data_type=expected_type,
                    category_id=row['food_category_id'], category=categories[row['food_category_id']],
                    publication_date=row['publication_date'], ndb_number=ndb_number,
                    record_checksum=row_checksum(evidence),
                    url=f'https://fdc.nal.usda.gov/food-details/{fdc_id}/nutrients')
                eligible += 1
            require(eligible == pin['eligible_rows'], 'Unexpected eligible USDA record count')
            counts[key] = dict(counter)
    require(archive_keys, 'No USDA archives')
    return dict(sorted(terms.items())), counts


def context(args):
    descriptor, seed, sources = load(args.descriptor), load(args.seed), load(args.sources)
    validate(seed, sources)
    terms, counts = read_archives(args.archives, descriptor)
    return descriptor, seed, sources, terms, counts


def bindings(args):
    return dict(schema_version=1, descriptor_checksum=digest(args.descriptor),
                base_seed_checksum=digest(args.seed), base_sources_checksum=digest(args.sources))


def name_index(seed):
    index = collections.defaultdict(set)
    for entry in seed['entries']:
        if entry['status'] == 'active':
            for name in [entry['label'], *entry['aliases']]:
                index[normalize(name)].add(entry['id'])
    return index


def extract(args):
    descriptor, seed, sources, terms, counts = context(args)
    index = name_index(seed)
    catalog = [{**term, 'name_matches': sorted(index[normalize(term['label'])])}
               for term in terms.values()]
    summary = dict(records=len(terms), food_rows_by_type=counts,
                   note='Candidates only; no name match approves equivalence or creates a canonical ID.')
    write_bundle(args.output, {'candidates.json': dict(schema_version=1, terms=catalog),
        'review.json': {**bindings(args), 'decisions': []}, 'summary.json': summary})
    return summary


def build(args):
    descriptor, seed, sources, terms, counts = context(args)
    review = load(args.review)
    require(set(review) == set(bindings(args)) | {'decisions'} and
            all(review[k] == v for k, v in bindings(args).items()), 'Stale/invalid USDA review bindings')
    decisions = review['decisions']
    require(isinstance(decisions, list) and decisions and
            all(type(d.get('approved')) is bool for d in decisions), 'Review needs boolean approvals')
    result, manifest = copy.deepcopy(seed), copy.deepcopy(sources)
    for pin in descriptor['archives']:
        source = pin['source']
        existing = next((s for s in manifest['sources'] if s['id'] == source['id']), None)
        require(existing is None or existing == source, 'Source ID already pins different metadata')
        if existing is None:
            manifest['sources'].append(source)
    entries, names = {e['id']: e for e in result['entries']}, name_index(result)
    seen, changes = set(), []
    for decision in decisions:
        if not decision['approved']:
            continue
        require(set(decision) <= {'approved', 'action', 'record_id', 'record_checksum',
                'canonical_id', 'relationship', 'canonical', 'note'}, 'Unknown decision field')
        rid, eid = decision['record_id'], decision['canonical_id']
        require(rid in terms and rid not in seen, 'Missing/repeated approved USDA record')
        seen.add(rid)
        term = terms[rid]
        require(decision['record_checksum'] == term['record_checksum'], 'Changed USDA row evidence')
        require(valid_text(decision.get('note'), 4000) and
                decision['relationship'] in RELATIONSHIPS, 'Missing rationale or invalid relationship')
        if decision['action'] == 'import':
            require(valid_id(eid) and uuid.UUID(eid).version == 4 and eid not in entries,
                    'Import requires a new persisted UUIDv4')
            wording = decision.get('canonical', {})
            require(set(wording) == {'label', 'aliases', 'description'} and
                    decision['relationship'] == 'exact' and isinstance(wording['aliases'], list) and
                    all(valid_text(n) for n in [wording['label'], *wording['aliases']]),
                    'Import requires explicit valid wording and exact source identity')
            for name in [wording['label'], *wording['aliases']]:
                require(not names[normalize(name)], 'Import label/alias collides with maintained identity')
                names[normalize(name)].add(eid)
            entry = dict(id=eid, **copy.deepcopy(wording), locale='en', status='active',
                         source_mappings=[], provenance=dict(origin='imported', references=[]))
            entries[eid] = entry; result['entries'].append(entry)
        else:
            require(decision['action'] == 'map' and eid in entries and
                    entries[eid]['status'] == 'active' and 'canonical' not in decision,
                    'Map must preserve an active canonical identity and wording')
            entry = entries[eid]
        source_id = term['source']
        if source_id not in entry['provenance']['references']:
            entry['provenance']['references'].append(source_id)
        mapping = dict(source=source_id, external_id=term['fdc_id'], relationship=decision['relationship'])
        require(not any(m['source'] == source_id and m['external_id'] == term['fdc_id'] and m != mapping
                        for m in entry['source_mappings']), 'Conflicting existing source relationship')
        if mapping not in entry['source_mappings']:
            entry['source_mappings'].append(mapping)
        changes.append(dict(record_id=rid, canonical_id=eid, relationship=decision['relationship'],
                            record_checksum=term['record_checksum'], note=decision['note']))
    require(changes, 'No approved decisions')
    warnings = validate(result, manifest)
    require(not warnings, 'Ambiguous active label/aliases; resolve before promotion')
    summary = dict(entries=len(result['entries']), approved_decisions=len(changes),
                   pending_decisions=len(decisions)-len(changes), alias_collisions=warnings)
    write_bundle(args.output, {'ingredients.json': result, 'sources.json': manifest,
                              'report.json': dict(summary, changes=changes)})
    return summary


def arguments(parser):
    parser.add_argument('--archives', type=Path, required=True)
    parser.add_argument('--descriptor', type=Path, default=ROOT/'data/vocabulary/adapters/usda.json')
    parser.add_argument('--seed', type=Path, default=ROOT/'data/vocabulary/ingredients.json')
    parser.add_argument('--sources', type=Path, default=ROOT/'data/vocabulary/sources.json')
    parser.add_argument('--output', type=Path, required=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    for command in ['extract', 'build']:
        child = commands.add_parser(command); arguments(child)
        if command == 'build':
            child.add_argument('--review', type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(extract(args) if args.command == 'extract' else build(args)))


if __name__ == '__main__':
    main()
