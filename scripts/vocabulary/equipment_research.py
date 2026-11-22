#!/usr/bin/env python3
"""Offline equipment candidate evaluation. Produces research files, never seeds."""
import argparse
import collections
import csv
import json
from pathlib import Path
import re

from foodon import digest, load, normalize, require, valid_text
from wikidata import item_values

NON_CONCEPTS = {'Q5', 'Q4167410', 'Q4167836', 'Q13406463'}


def snapshot(path, descriptor):
    require(path.stat().st_size == descriptor['bytes'] and digest(path) == descriptor['source']['checksum'],
            'Snapshot bytes/checksum differ from capture')
    require(descriptor['schema_version'] == 1 and descriptor['source']['license'] == 'CC0-1.0',
            'Unsupported equipment capture')
    data = load(path)
    require(data['schema_version'] == 1, 'Unsupported snapshot')
    entities = {}
    for entity in data['entities']:
        qid = entity['id']
        require(re.fullmatch(r'Q[1-9][0-9]*', qid) and qid not in entities and
                entity['type'] == 'item' and entity['lastrevid'] == descriptor['revisions'].get(qid),
                'Invalid, duplicate or mismatched entity revision')
        entities[qid] = entity
    require(set(entities) == set(descriptor['revisions']), 'Incomplete capture')
    return entities


def source_text(entity):
    return (entity['labels'].get('en', {}).get('value', ''),
            entity['descriptions'].get('en', {}).get('value', ''),
            [row['value'] for row in entity['aliases'].get('en', [])])


def disposition(entity, choice=None):
    label, description, aliases = source_text(entity)
    types = set(item_values(entity, 'P31'))
    flags = []
    if not valid_text(label):
        flags.append('missing_or_invalid_english_label')
    if not description:
        flags.append('missing_english_description')
    if entity.get('redirect'):
        flags.append('unresolved_redirect')
    if types & NON_CONCEPTS:
        flags.append('person_or_wikimedia_page')
    if '&' in label or re.search(r'\b(product|accessor(?:y|ies)|set)\b', label, re.I):
        flags.append('catalogue_or_grouped_label')
    if item_values(entity, 'P1889'):
        flags.append('upstream_distinguishes_other_entities')
    if choice:
        require(not set(flags) & {'missing_or_invalid_english_label', 'unresolved_redirect', 'person_or_wikimedia_page'},
                f'Ineligible recommendation: {entity["id"]}')
        return 'recommended_candidate', choice['reason'], flags
    text = (label + ' ' + description).lower()
    if 'person_or_wikimedia_page' in flags or re.search(
            r'\b(olympic|paralympic|olympics|monument|inscription|museum|artifact|artefact|ancient|archaeolog|liturg|ceremonial|votive|communion|baptismal|holy grail|haniwa)\b', text):
        return 'exclude_from_kitchen_seed', 'Historical, ceremonial, individual-object or non-equipment record.', flags
    if re.search(r'\b(brand|company|corporation|brewery|historic cider mill|product range|vending machine|laboratory|pharmaceutical|cosmetic|contraceptive|callus|bath towel|toilet towel|bathroom)\b', text):
        return 'exclude_from_kitchen_seed', 'Brand, industrial/laboratory or unrelated household scope.', flags
    if 'missing_or_invalid_english_label' in flags:
        return 'hold_for_review', 'No usable English label; no invented translation.', flags
    if 'catalogue_or_grouped_label' in flags:
        return 'hold_for_review', 'Grouped catalogue term, accessory, replacement part or product set needs identity/scope review.', flags
    return 'hold_for_review', 'Not selected in this initial kitchen shortlist; specialist, serving, historical or ambiguous scope needs further review.', flags


def lexical_index(entities):
    result = collections.defaultdict(set)
    for qid, entity in entities.items():
        label, _, aliases = source_text(entity)
        for text in [label, *aliases]:
            if valid_text(text):
                result[normalize(text)].add(qid)
    return result


def coverage(benchmark, entities, choices):
    index = lexical_index(entities)
    proposed = collections.defaultdict(set)
    for qid, choice in choices.items():
        proposed[normalize(choice['label'])].add(qid)
    result = []
    for row in benchmark:
        ids = sorted(set().union(*(index.get(normalize(term), set()) for term in [row['label'], *row.get('aliases', [])])))
        editorial = sorted(set().union(*(proposed.get(normalize(term), set()) for term in [row['label'], *row.get('aliases', [])])))
        selected = sorted({qid for qid in ids if qid in choices} | set(editorial))
        # This grades lexical discovery, never exact semantic equivalence.
        status = ('shortlisted_name_match' if selected else
                  'unreviewed_or_excluded_name_match' if ids else 'no_name_match_in_capture')
        result.append(dict(label=row['label'], reference=row['reference'], status=status,
                           source_ids=ids, proposed_label_ids=editorial, shortlisted_source_ids=selected,
                           match_basis='source_name_or_alias' if ids else 'proposed_label_only' if editorial else 'none'))
    return result


def write_csv(path, rows, fields):
    with path.open('w', newline='', encoding='utf-8-sig') as stream:
        writer = csv.DictWriter(stream, fieldnames=fields)
        writer.writeheader()
        for row in rows:
            values = {key: json.dumps(row.get(key, ''), ensure_ascii=False) if isinstance(row.get(key), (list, dict)) else row.get(key, '') for key in fields}
            # Raw source JSON stays unchanged; downloadable spreadsheet cells
            # cannot execute source-supplied formulas on opening.
            writer.writerow({key: "'" + value if isinstance(value, str) and value.startswith(('=', '+', '-', '@')) else value for key, value in values.items()})


def evaluate(args):
    descriptor, policy = load(args.descriptor), load(args.policy)
    entities = snapshot(args.snapshot, descriptor)
    require(policy['source_checksum'] == descriptor['source']['checksum'], 'Review belongs to a different capture')
    choices = policy['choices']
    require(policy['schema_version'] == 1, 'Unsupported review policy')
    require(set(choices) <= set(entities), 'Review refers to uncaptured entities')
    require(all(valid_text(row['label']) and valid_text(row['reason'], 1000) and
                row['tier'] in {'core', 'specialist', 'scope_review'} and
                row['family'] in {'cookware', 'bakeware', 'preparation_tools', 'appliances',
                                  'measuring_tools', 'storage', 'bar_tools', 'kitchen_supplies'}
                for row in choices.values()), 'Invalid shortlist metadata')
    labels = [normalize(row['label']) for row in choices.values()]
    require(len(labels) == len(set(labels)), 'Shortlist has duplicate normalized primary labels')
    rows = []
    for qid, entity in entities.items():
        label, description, aliases = source_text(entity)
        choice = choices.get(qid)
        status, reason, flags = disposition(entity, choice)
        rows.append(dict(source_id=qid, revision=entity['lastrevid'], source_label=label,
            proposed_label=choice['label'] if choice else '', family=choice['family'] if choice else '',
            tier=choice['tier'] if choice else '', status=status, reason=reason, description=description,
            source_aliases=aliases, flags=flags, instance_of=item_values(entity, 'P31'),
            subclass_of=item_values(entity, 'P279'), different_from=item_values(entity, 'P1889'),
            revision_url=f'https://www.wikidata.org/w/index.php?title={qid}&oldid={entity["lastrevid"]}'))
    rows.sort(key=lambda row: (row['source_label'].casefold(), int(row['source_id'][1:])))
    label_index = collections.defaultdict(list)
    for row in rows:
        if row['source_label']:
            label_index[normalize(row['source_label'])].append(row['source_id'])
    collisions = [dict(name=name, source_ids=ids) for name, ids in sorted(label_index.items()) if len(ids) > 1]
    alias_collisions = [dict(name=name, source_ids=sorted(ids)) for name, ids in sorted(lexical_index(entities).items()) if len(ids) > 1]
    checked = coverage(load(args.benchmark), entities, choices)
    summary = dict(schema_version=1, source_checksum=descriptor['source']['checksum'],
        policy_checksum=digest(args.policy), benchmark_checksum=digest(args.benchmark),
        discovery_records=descriptor['raw_records'], detailed_records=len(rows),
        instance_records_outside_detailed_capture=descriptor['raw_records']-len(rows),
        english_labels=sum(bool(row['source_label']) for row in rows),
        missing_english_labels=sum(not row['source_label'] for row in rows),
        missing_english_descriptions=sum(not row['description'] for row in rows),
        dispositions=dict(collections.Counter(row['status'] for row in rows)),
        shortlist_families=dict(collections.Counter(row['family'] for row in rows if row['proposed_label'])),
        shortlist_tiers=dict(collections.Counter(row['tier'] for row in rows if row['proposed_label'])),
        renamed_shortlist_labels=sum(row['proposed_label'] != row['source_label'] for row in rows if row['proposed_label']),
        shortlist_missing_descriptions=sum(not row['description'] for row in rows if row['proposed_label']),
        duplicate_primary_name_groups=len(collisions), shared_label_or_alias_groups=len(alias_collisions),
        coverage_by_reference={ref: dict(collections.Counter(row['status'] for row in checked if row['reference'] == ref)) for ref in sorted({row['reference'] for row in checked})},
        coverage_match_basis={ref: dict(collections.Counter(row['match_basis'] for row in checked if row['reference'] == ref)) for ref in sorted({row['reference'] for row in checked})},
        limitations=['Bounded hierarchy discovery, not all Wikidata equipment.',
                    'Per-entity revisions, not an atomic global dump.',
                    'Policy-assisted triage and an explicitly curated shortlist, not semantic certification of every record.',
                    'Lexical benchmark matches are not identity approvals; no match does not prove absence from Wikidata.',
                    'Upstream aliases are research evidence only; none approved for import.',
                    'No production seed, API, schema, recipe or historical data changes.'])
    args.output.mkdir(parents=True, exist_ok=True)
    fields = list(rows[0])
    write_csv(args.output/'all-candidates.csv', rows, fields)
    write_csv(args.output/'shortlist.csv', [row for row in rows if row['proposed_label']], fields)
    write_csv(args.output/'duplicate-labels.csv', collisions, ['name', 'source_ids'])
    write_csv(args.output/'shared-labels-and-aliases.csv', alias_collisions, ['name', 'source_ids'])
    write_csv(args.output/'coverage.csv', checked, ['reference', 'label', 'status', 'source_ids', 'proposed_label_ids', 'shortlisted_source_ids', 'match_basis'])
    (args.output/'summary.json').write_text(json.dumps(summary, ensure_ascii=False, indent=2)+'\n')
    print(json.dumps(summary, indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ['snapshot', 'descriptor', 'policy', 'benchmark', 'output']:
        parser.add_argument('--'+key, type=Path, required=True)
    evaluate(parser.parse_args())
