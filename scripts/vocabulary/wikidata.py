#!/usr/bin/env python3
"""Offline, revision-pinned Wikidata extraction and reviewed canonical seed builds."""
import argparse
import collections
import copy
import json
from pathlib import Path
import re
import uuid

from foodon import ROOT, RELATIONSHIPS, digest, load, normalize, require, valid_id, valid_text, validate, write_bundle

QID = re.compile(r'Q[1-9][0-9]*\Z')
NONFOOD_TYPES = {'Q16521', 'Q4167410', 'Q4167836', 'Q13406463'}


def item_values(entity, prop):
    """Wikidata truthy semantics: preferred values supersede normal; never deprecated."""
    statements = entity['claims'].get(prop, [])
    rank = 'preferred' if any(s['rank'] == 'preferred' for s in statements) else 'normal'
    result = []
    for statement in statements:
        if statement['rank'] != rank:
            continue
        snak = statement['mainsnak']
        if snak['snaktype'] == 'value' and snak.get('datavalue', {}).get('type') == 'wikibase-entityid':
            value = snak['datavalue']['value'].get('id', '')
            if QID.fullmatch(value):
                result.append(value)
    return list(dict.fromkeys(result))


def read_snapshot(snapshot, descriptor):
    require(descriptor['schema_version'] == 1 and descriptor['source']['license'] == 'CC0-1.0',
            'Unsupported Wikidata descriptor')
    require(snapshot.stat().st_size == descriptor['bytes'] and digest(snapshot) == descriptor['source']['checksum'],
            'Snapshot bytes/checksum differ from descriptor')
    data = load(snapshot)
    require(set(data) == {'schema_version', 'entities'} and data['schema_version'] == 1,
            'Invalid snapshot header')
    revisions = descriptor['revisions']
    require(all(QID.fullmatch(q) and type(r) is int and r > 0 for q, r in revisions.items()),
            'Invalid pinned revision registry')
    outside = set(descriptor['outside_discovery_entities'])
    require(outside <= set(revisions), 'Incomplete ingredient scope registry')
    ingredient_entities = set(revisions) - outside
    terms = {}
    for entity in data['entities']:
        qid = entity['id']
        require(QID.fullmatch(qid) and qid not in terms and entity['type'] == 'item' and
                type(entity['lastrevid']) is int and entity['lastrevid'] == revisions.get(qid),
                'Invalid/duplicate/mismatched entity revision')
        for prop, statements in entity['claims'].items():
            require(re.fullmatch(r'P[1-9][0-9]*', prop) and isinstance(statements, list), 'Invalid statements')
            require(all(s['rank'] in {'normal', 'preferred', 'deprecated'} and
                        s['mainsnak']['property'] == prop for s in statements), 'Invalid statement rank/property')
        label = entity['labels'].get('en', {}).get('value', '')
        description = entity['descriptions'].get('en', {}).get('value', '')
        aliases = [a['value'] for a in entity['aliases'].get('en', []) if a['language'] == 'en']
        instances = item_values(entity, 'P31')
        flags = []
        if not valid_text(label):
            flags.append('missing_or_invalid_english_label')
        if not description:
            flags.append('missing_english_description')
        if entity.get('redirect'):
            flags.append('redirect_requires_resolution')
        if set(instances) & NONFOOD_TYPES or item_values(entity, 'P105'):
            flags.append('taxon_or_wikimedia_page')
        if qid not in ingredient_entities or qid == 'Q25403900':
            flags.append('outside_ingredient_discovery')
        terms[qid] = dict(id=qid, revision=entity['lastrevid'], label=label, description=description,
                          aliases=aliases, instance_of=instances, subclass_of=item_values(entity, 'P279'),
                          flags=flags, eligible=not any(f != 'missing_english_description' for f in flags),
                          revision_url=f'https://www.wikidata.org/w/index.php?title={qid}&oldid={entity["lastrevid"]}',
                          statements=entity['claims'])
    require(set(terms) == set(revisions), 'Snapshot omits pinned entities')
    return terms


def context(args):
    descriptor = load(args.descriptor)
    seed, sources = load(args.seed), load(args.sources)
    validate(seed, sources)
    return descriptor, seed, sources, read_snapshot(args.snapshot, descriptor)


def bindings(args, descriptor):
    return dict(schema_version=1, source_checksum=descriptor['source']['checksum'],
                descriptor_checksum=digest(args.descriptor), base_seed_checksum=digest(args.seed),
                base_sources_checksum=digest(args.sources))


def name_index(seed):
    index = collections.defaultdict(set)
    for e in seed['entries']:
        if e['status'] == 'active':
            for name in [e['label'], *e['aliases']]:
                index[normalize(name)].add(e['id'])
    return index


def extract(args):
    descriptor, seed, sources, terms = context(args)
    index = name_index(seed)
    by_id = {e['id']: e for e in seed['entries']}
    proposals, catalog, overlaps = [], [], collections.defaultdict(list)
    for qid, term in terms.items():
        matches = set().union(*(index[normalize(n)] for n in [term['label'], *term['aliases']] if n))
        row = {**term, 'name_matches': sorted(matches)}
        catalog.append(row)
        for eid in matches:
            overlaps[eid].append(qid)
            proposals.append(dict(approved=False, action='map', term_id=qid, revision=term['revision'],
                                  canonical_id=eid, relationship=None,
                                  note='Name overlap is discovery evidence; review identity/part/form before choosing a relationship.'))
    review = {**bindings(args, descriptor), 'decisions': proposals}
    # Every active maintained identity is accounted for, including unmatched names.
    coverage = [dict(canonical_id=e['id'], label=e['label'], aliases=e['aliases'],
                     candidates=sorted(overlaps[e['id']]),
                     status='candidate_names' if overlaps[e['id']] else 'no_name_match_in_snapshot')
                for e in seed['entries'] if e['status'] == 'active']
    report = dict(entities=len(terms), eligible=sum(t['eligible'] for t in terms.values()),
                  active_ingredients=len(coverage), with_name_candidates=sum(bool(r['candidates']) for r in coverage),
                  pending_mappings=len(proposals),
                  note='All names screened against this bounded snapshot; unmatched does not mean absent from Wikidata.')
    write_bundle(args.output, {'candidates.json': dict(schema_version=1, terms=catalog),
                              'review.json': review, 'seed-name-review.json': coverage, 'report.json': report})
    return report


def build(args):
    descriptor, seed, sources, terms = context(args)
    review = load(args.review)
    require(set(review) == set(bindings(args, descriptor)) | {'decisions'}, 'Invalid review fields')
    require(all(review[k] == v for k, v in bindings(args, descriptor).items()), 'Stale review inputs or descriptor')
    require(isinstance(review['decisions'], list) and
            all(type(d.get('approved')) is bool for d in review['decisions']), 'approved must be a JSON boolean')
    approved = [d for d in review['decisions'] if d['approved']]
    require(approved, 'No approved decisions')
    result, manifest = copy.deepcopy(seed), copy.deepcopy(sources)
    source = descriptor['source']
    existing_source = next((s for s in manifest['sources'] if s['id'] == source['id']), None)
    require(existing_source is None or existing_source == source, 'Source ID already pins different metadata')
    if existing_source is None:
        manifest['sources'].append(source)
    entries = {e['id']: e for e in result['entries']}
    index = name_index(result)
    seen, changes = set(), []
    for d in approved:
        require(set(d) <= {'approved', 'action', 'term_id', 'revision', 'canonical_id',
                          'relationship', 'canonical', 'note'}, 'Unknown decision field')
        qid, eid = d['term_id'], d['canonical_id']
        require(qid in terms and type(d['revision']) is int and d['revision'] == terms[qid]['revision'],
                'Decision targets missing entity or another revision')
        require((eid, qid) not in seen and isinstance(d.get('note'), str) and d['note'].strip(),
                'Duplicate decision or missing review rationale')
        seen.add((eid, qid))
        term, relationship = terms[qid], d['relationship']
        require(relationship in RELATIONSHIPS and not term.get('redirect'), 'Invalid relationship or unresolved redirect')
        hard_flags = set(term['flags']) - {'outside_ingredient_discovery', 'missing_english_description'}
        require(not hard_flags, 'Taxon, Wikimedia page or invalid label cannot be a culinary mapping')
        if d['action'] == 'import':
            require(term['eligible'] and valid_id(eid) and uuid.UUID(eid).version == 4 and eid not in entries,
                    'Import requires eligible entity and a new persisted UUIDv4')
            require(relationship == 'exact' and set(d.get('canonical', {})) == {'label', 'aliases', 'description'},
                    'Import requires explicit reviewed wording and exact source identity')
            wording = d['canonical']
            require(isinstance(wording['aliases'], list) and all(valid_text(n) for n in [wording['label'], *wording['aliases']]),
                    'Invalid reviewed import wording')
            for name in [wording['label'], *wording['aliases']]:
                require(not index[normalize(name)], 'Import label/alias collides with an existing identity; review a mapping')
                index[normalize(name)].add(eid)
            entry = dict(id=eid, **copy.deepcopy(wording), locale='en', status='active',
                         source_mappings=[], provenance=dict(origin='imported', references=[]))
            entries[eid] = entry
            result['entries'].append(entry)
        else:
            require(d['action'] == 'map' and eid in entries and 'canonical' not in d,
                    'Mapping must preserve an existing canonical identity and wording')
            entry = entries[eid]
        refs = entry['provenance']['references']
        if source['id'] not in refs:
            refs.append(source['id'])
        mapping = dict(source=source['id'], external_id=qid, relationship=relationship)
        require(not any(m['source'] == source['id'] and m['external_id'] == qid and m != mapping
                        for m in entry['source_mappings']), 'Conflicting existing relationship')
        if mapping not in entry['source_mappings']:
            entry['source_mappings'].append(mapping)
        changes.append(dict(canonical_id=eid, term_id=qid, revision=d['revision'], relationship=relationship,
                            note=d['note'], revision_url=term['revision_url']))
    warnings = validate(result, manifest)
    require(not warnings, 'Build introduces or retains ambiguous active label/aliases; resolve before promotion')
    report = dict(entries=len(result['entries']), approved_decisions=len(approved),
                  pending_decisions=len(review['decisions']) - len(approved), alias_collisions=warnings,
                  changes=changes, selected_entities=[terms[q] for q in sorted({d['term_id'] for d in approved})])
    write_bundle(args.output, {'ingredients.json': result, 'sources.json': manifest, 'report.json': report})
    return {k: report[k] for k in ('entries', 'approved_decisions', 'pending_decisions', 'alias_collisions')}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    for name in ('extract', 'build'):
        cmd = commands.add_parser(name)
        cmd.add_argument('--snapshot', type=Path, required=True)
        cmd.add_argument('--descriptor', type=Path, default=ROOT / 'data/vocabulary/adapters/wikidata.json')
        cmd.add_argument('--seed', type=Path, default=ROOT / 'data/vocabulary/ingredients.json')
        cmd.add_argument('--sources', type=Path, default=ROOT / 'data/vocabulary/sources.json')
        cmd.add_argument('--output', type=Path, required=True)
        if name == 'build':
            cmd.add_argument('--review', type=Path, required=True)
    args = parser.parse_args()
    try:
        print(json.dumps((extract if args.command == 'extract' else build)(args)))
    except (ValueError, KeyError, TypeError, OSError) as error:
        parser.exit(1, f'Wikidata adapter failed: {error}\n')


if __name__ == '__main__':
    main()
