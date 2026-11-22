#!/usr/bin/env python3
"""Offline FoodOn RDF/XML adapter: extract review candidates, then build a seed.

No network requests, OWL import resolution, database writes or inferred identity
merges. The reviewed source descriptor pins every input ontology by SHA-256.
"""
import argparse
import collections
import copy
import hashlib
import json
from pathlib import Path
import re
import tempfile
import unicodedata
import uuid
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
RDF = 'http://www.w3.org/1999/02/22-rdf-syntax-ns#'
RDFS = 'http://www.w3.org/2000/01/rdf-schema#'
OWL = 'http://www.w3.org/2002/07/owl#'
OBO = 'http://purl.obolibrary.org/obo/'
OIO = 'http://www.geneontology.org/formats/oboInOwl#'
XML_LANG = '{http://www.w3.org/XML/1998/namespace}lang'
RELATIONSHIPS = {'exact', 'broader', 'narrower', 'related'}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def load(path):
    return json.loads(Path(path).read_text(encoding='utf-8'))


def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def normalize(text):
    # Match vocabulary::normalize: NFKC + lowercase + collapsed whitespace.
    return ' '.join(unicodedata.normalize('NFKC', text).lower().split())


def valid_text(value, limit=200):
    return (isinstance(value, str) and bool(value.strip()) and len(value) <= limit
            and not any(unicodedata.category(c) == 'Cc' for c in value))


def valid_id(value):
    try:
        return str(uuid.UUID(value)) == value and uuid.UUID(value).int != 0
    except (ValueError, TypeError, AttributeError):
        return False


def validate(seed, manifest):
    """Early seed-contract checks; Rust --validate-only is the authoritative gate."""
    require(set(seed) == {'schema_version', 'dimension', 'entries'} and
            seed['schema_version'] == 1 and seed['dimension'] == 'ingredient', 'Invalid seed header')
    require(set(manifest) == {'schema_version', 'sources'} and manifest['schema_version'] == 1,
            'Invalid source manifest header')
    source_ids = set()
    for source in manifest['sources']:
        require(set(source) == {'id', 'url', 'version', 'checksum', 'license', 'attribution'},
                'Invalid source fields')
        require(valid_text(source['id']) and source['id'] not in source_ids and
                isinstance(source['url'], str) and source['url'].startswith('https://') and
                re.fullmatch(r'[a-fA-F0-9]{64}', source['checksum']) and
                all(isinstance(source[k], str) and source[k].strip()
                    for k in ('version', 'license', 'attribution')), 'Invalid source metadata')
        source_ids.add(source['id'])
    require(1 <= len(seed['entries']) <= 100000, 'Expected 1–100000 entries')
    ids, names, warnings, exact = set(), {}, [], {}
    for entry in seed['entries']:
        require(set(entry) == {'id', 'label', 'locale', 'aliases', 'description', 'status',
                               'source_mappings', 'provenance'}, 'Invalid canonical entry fields')
        eid = entry['id']
        require(valid_id(eid) and eid not in ids and entry['locale'] == 'en' and
                entry['status'] in {'active', 'retired'} and valid_text(entry['label']) and
                isinstance(entry['description'], str) and len(entry['description']) <= 2000 and
                isinstance(entry['aliases'], list) and len(entry['aliases']) <= 100,
                f'Invalid or duplicate entry: {eid}')
        ids.add(eid)
        local = set()
        for name in [entry['label'], *entry['aliases']]:
            require(valid_text(name), f'Invalid label/alias in {eid}')
            key = normalize(name)
            require(key not in local, f'Repeated label/alias in {eid}')
            local.add(key)
            if entry['status'] == 'active':
                if key in names and names[key] != eid:
                    warnings.append({'name': name, 'ids': [names[key], eid]})
                names[key] = eid
        provenance = entry['provenance']
        require(set(provenance) == {'origin', 'references'} and
                provenance['origin'] in {'curated', 'imported'}, f'Invalid provenance: {eid}')
        refs = provenance['references']
        require(isinstance(refs, list) and len(set(refs)) == len(refs) and
                set(refs) <= source_ids and (refs or provenance['origin'] != 'imported'),
                f'Invalid source references: {eid}')
        seen = set()
        for mapping in entry['source_mappings']:
            require(set(mapping) == {'source', 'external_id', 'relationship'}, 'Invalid mapping fields')
            key = (mapping['source'], mapping['external_id'], mapping['relationship'])
            require(key not in seen and mapping['source'] in refs and
                    valid_text(mapping['external_id']) and mapping['relationship'] in RELATIONSHIPS,
                    f'Invalid source mapping: {eid}')
            seen.add(key)
            if mapping['relationship'] == 'exact':
                identity = (mapping['source'], mapping['external_id'])
                require(identity not in exact or exact[identity] == eid,
                        f'Exact source term assigned to distinct canonical IDs: {identity}')
                exact[identity] = eid
    return warnings


def term_id(iri):
    if iri and iri.startswith(OBO) and re.fullmatch(r'FOODON_[0-9]{8}', iri[len(OBO):]):
        return iri[len(OBO):]
    return None


def english(literals):
    return list(dict.fromkeys(r['value'] for r in literals
                             if not r['language'] or r['language'].lower().split('-')[0] == 'en'))


def proposed_wording(item):
    seen, aliases = {normalize(item['label'])}, []
    for value in item['synonyms']['exact']:
        if normalize(value) not in seen:
            aliases.append(value)
            seen.add(normalize(value))
    return {'label': item['label'], 'aliases': aliases,
            'description': item['definitions'][0] if item['definitions'] else ''}


def literal(node):
    return {'value': ''.join(node.itertext()).strip(), 'language': node.get(XML_LANG, ''),
            'datatype': node.get('{' + RDF + '}datatype', '')}


def annotations(node):
    rows = []
    for child in node:
        if child.tag in {'{' + RDFS + '}subClassOf', '{' + OWL + '}equivalentClass'}:
            continue
        if not list(child):
            rows.append({'property': child.tag[1:].replace('}', ''),
                         **literal(child), 'resource': child.get('{' + RDF + '}resource', '')})
    return rows


def read_ontology(path, descriptor):
    require(descriptor['schema_version'] == 1, 'Unsupported adapter descriptor')
    require(Path(path).stat().st_size == descriptor['bytes'], 'Ontology byte length differs from pin')
    require(digest(path) == descriptor['source']['checksum'], 'Ontology SHA-256 differs from pin')
    # ElementTree never resolves external imports. Explicitly forbid XML entities/DTDs too.
    with Path(path).open('rb') as stream:
        previous = b''
        while block := stream.read(1024 * 1024):
            part = previous + block
            require(b'<!DOCTYPE' not in part and b'<!ENTITY' not in part, 'DTD/entities are not supported')
            previous = block[-16:]
    terms, axioms, metadata, declared = {}, collections.defaultdict(list), [], set()
    version_iris = set()
    stack = []
    for event, node in ET.iterparse(path, events=('start', 'end')):
        if event == 'start':
            stack.append(node)
            continue
        if len(stack) == 2:
            if node.tag == '{' + OWL + '}Ontology':
                metadata.extend(annotations(node))
                version_iris.update(c.get('{' + RDF + '}resource') for c in node
                                    if c.tag == '{' + OWL + '}versionIRI')
            tid = term_id(node.get('{' + RDF + '}about'))
            is_class = (node.tag == '{' + OWL + '}Class' or any(
                c.tag == '{' + RDF + '}type' and c.get('{' + RDF + '}resource') == OWL + 'Class'
                for c in node))
            if tid and is_class:
                declared.add(tid)
            if tid and (is_class or node.tag == '{' + RDF + '}Description'):
                parents, parent_iris = set(), set()
                for child in node:
                    if child.tag in {'{' + RDFS + '}subClassOf', '{' + OWL + '}equivalentClass'}:
                        iri = child.get('{' + RDF + '}resource')
                        if iri:
                            parent_iris.add(iri)
                        parent = term_id(iri)
                        if parent:
                            parents.add(parent)
                        # Named conjuncts are superclasses; fillers of restrictions are NOT.
                        for intersection in child.findall('./{' + OWL + '}Class/{' + OWL + '}intersectionOf'):
                            for conjunct in intersection:
                                iri = conjunct.get('{' + RDF + '}about')
                                if iri and conjunct.tag in {'{' + RDF + '}Description', '{' + OWL + '}Class'}:
                                    parent_iris.add(iri)
                                    parent = term_id(iri)
                                    if parent:
                                        parents.add(parent)
                item = {'id': tid, 'iri': OBO + tid, 'parents': sorted(parents), 'parent_iris': sorted(parent_iris),
                        'annotations': annotations(node)}
                if tid in terms:
                    # RDF/XML can split one subject across several class elements.
                    item['parents'] = sorted(set(item['parents'] + terms[tid]['parents']))
                    item['parent_iris'] = sorted(set(item['parent_iris'] + terms[tid]['parent_iris']))
                    item['annotations'] = terms[tid]['annotations'] + item['annotations']
                terms[tid] = item
            elif node.tag == '{' + OWL + '}Axiom':
                source = node.find('{' + OWL + '}annotatedSource')
                tid = term_id(source.get('{' + RDF + '}resource')) if source is not None else None
                if tid:
                    axioms[tid].append(annotations(node))
            stack[0].remove(node)
            node.clear()
        stack.pop()
    require(descriptor['version_iri'] in version_iris, 'Ontology version IRI differs from pin')
    terms = {tid: item for tid, item in terms.items() if tid in declared}
    root = descriptor['ingredient_root']
    require(root in terms, 'Food material root is absent; inspect source hierarchy')
    exceptions = descriptor.get('reviewed_ingredient_terms', {})
    require(isinstance(exceptions, dict) and all(tid in terms and tid != root and valid_text(reason, 2000)
            for tid, reason in exceptions.items()), 'Invalid reviewed ingredient scope exceptions')
    children = collections.defaultdict(set)
    for tid, item in terms.items():
        for parent in item['parents']:
            children[parent].add(tid)
    eligible, pending = set(), [root]
    while pending:
        tid = pending.pop()
        if tid not in eligible:
            eligible.add(tid)
            pending.extend(children[tid] - eligible)
    for tid, item in terms.items():
        def values(prop):
            return [a for a in item['annotations'] if a['property'] == prop and not a['resource']]
        labels = english(values(RDFS + 'label'))
        definitions = english(values(OBO + 'IAO_0000115'))
        item.update(label=labels[0] if labels else '', definitions=definitions,
                    synonyms={scope: english(values(OIO + 'has' + scope.title() + 'Synonym'))
                              for scope in ('exact', 'broad', 'narrow', 'related')},
                    axiom_annotations=axioms[tid])
        deprecated = any(a['value'].lower() in {'true', '1'} for a in values(OWL + 'deprecated'))
        item['eligible'] = (tid in eligible or tid in exceptions) and tid != root and not deprecated and len(labels) == 1
        item['scope_review'] = exceptions.get(tid)
        item['flags'] = []
        for condition, flag in ((deprecated, 'deprecated'), (tid not in eligible or tid == root, 'outside_food_material_hierarchy'),
                                (tid in exceptions, 'reviewed_ingredient_scope_exception'),
                                (len(labels) != 1, 'missing_or_ambiguous_english_label'),
                                (bool(re.match(r'^\d+\s*-|.*\(efsa foodex2\)', item['label'], re.I)), 'technical_label')):
            if condition:
                item['flags'].append(flag)
        item['canonical_proposal'] = proposed_wording(item)
        proposal = item['canonical_proposal']
        if (not valid_text(proposal['label']) or len(proposal['aliases']) > 100 or
                any(not valid_text(a) for a in proposal['aliases']) or len(proposal['description']) > 2000):
            item['flags'].append('canonical_text_requires_editing')
    return dict(sorted(terms.items())), metadata


def context(args):
    descriptor = load(args.descriptor)
    seed, sources = load(args.seed), load(args.sources)
    validate(seed, sources)
    terms, metadata = read_ontology(args.owl, descriptor)
    return descriptor, seed, sources, terms, metadata


def write_bundle(path, files):
    path = Path(path).resolve()
    require(not path.exists(), 'Output directory already exists; choose a fresh directory')
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.foodon-', dir=path.parent) as temporary:
        stage = Path(temporary) / 'bundle'
        stage.mkdir()
        for name, value in files.items():
            (stage / name).write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
        stage.rename(path)


def extract(args):
    descriptor, seed, sources, terms, metadata = context(args)
    by_label = collections.defaultdict(list)
    for entry in seed['entries']:
        by_label[entry['label']].append(entry)
    proposals = []
    for row in load(args.pilot):
        if row['status'] != 'E':
            continue
        tid = row['source_id']
        require(tid in terms, f'Pilot term absent from pinned ontology: {tid}')
        require(len(by_label[row['ingredient']]) == 1, f'Pilot canonical label missing/ambiguous: {row["ingredient"]}')
        proposals.append({'approved': False, 'action': 'map', 'term_id': tid,
                          'canonical_id': by_label[row['ingredient']][0]['id'],
                          'relationship': None, 'note': row['reason']})
    decisions = {'schema_version': 1, 'source_checksum': descriptor['source']['checksum'],
                 'descriptor_checksum': digest(args.descriptor),
                 'base_seed_checksum': digest(args.seed), 'base_sources_checksum': digest(args.sources),
                 'decisions': proposals}
    catalog = {'schema_version': 1, 'source': descriptor['source'], 'ontology_annotations': metadata,
               'terms': list(terms.values())}
    report = {'terms': len(terms), 'eligible_terms': sum(t['eligible'] for t in terms.values()),
              'pending_pilot_mappings': len(proposals),
              'note': 'Coverage E is a discovery judgment, not exact equivalence. Choose mapping relationships explicitly; nothing is approved automatically.'}
    write_bundle(args.output, {'candidates.json': catalog, 'review.json': decisions, 'report.json': report})
    return report


def build(args):
    descriptor, seed, sources, terms, metadata = context(args)
    review = load(args.review)
    require(set(review) == {'schema_version', 'source_checksum', 'descriptor_checksum', 'base_seed_checksum',
                            'base_sources_checksum', 'decisions'}, 'Invalid review fields')
    require(review['schema_version'] == 1 and review['source_checksum'] == descriptor['source']['checksum'],
            'Review targets a different source snapshot')
    require(review['descriptor_checksum'] == digest(args.descriptor), 'Adapter descriptor changed since review')
    require(review['base_seed_checksum'] == digest(args.seed) and
            review['base_sources_checksum'] == digest(args.sources), 'Base inputs changed since review was prepared')
    result, manifest = copy.deepcopy(seed), copy.deepcopy(sources)
    entries = {e['id']: e for e in result['entries']}
    source = descriptor['source']
    approved = [d for d in review['decisions'] if d.get('approved') is True]
    require(all(type(d.get('approved')) is bool for d in review['decisions']), 'approved must be a JSON boolean')
    require(approved, 'No approved decisions; review the candidates before building')
    existing_source = next((s for s in manifest['sources'] if s['id'] == source['id']), None)
    require(existing_source is None or existing_source == source,
            'Source ID already represents different metadata; use a reviewed snapshot-specific source ID')
    if existing_source is None:
        manifest['sources'].append(source)
    seen, changes = set(), []
    for decision in approved:
        require(set(decision) <= {'approved', 'action', 'term_id', 'canonical_id', 'relationship',
                                  'canonical', 'note'}, 'Unknown decision field')
        tid, eid = decision['term_id'], decision['canonical_id']
        require((eid, tid) not in seen, 'Repeated approved canonical/term decision')
        seen.add((eid, tid))
        require(tid in terms and terms[tid]['eligible'], f'Not an eligible active FoodOn ingredient term: {tid}')
        relationship = decision['relationship']
        require(relationship in RELATIONSHIPS, 'Choose an explicit reviewed relationship')
        if decision['action'] == 'import':
            require(valid_id(eid) and uuid.UUID(eid).version == 4 and eid not in entries,
                    'New imports require a new, persisted UUIDv4, not a derived or existing ID')
            wording = copy.deepcopy(terms[tid]['canonical_proposal'])
            overrides = copy.deepcopy(decision.get('canonical', {}))
            require(set(overrides) <= {'label', 'aliases', 'description', 'status'}, 'Invalid canonical wording overrides')
            status = overrides.pop('status', 'active')
            require(status in {'active', 'retired'}, 'Invalid canonical status')
            require(status != 'retired' or bool(decision.get('note', '').strip()),
                    'Retained retired imports require an editorial reason')
            wording.update(overrides)
            require(relationship == 'exact', 'A new source-derived entry must represent the selected term exactly')
            entry = {'id': eid, **wording, 'locale': 'en', 'status': status, 'source_mappings': [],
                     'provenance': {'origin': 'imported', 'references': []}}
            result['entries'].append(entry)
            entries[eid] = entry
        else:
            require(decision['action'] == 'map' and eid in entries, 'Map requires an existing canonical ID')
            require('canonical' not in decision, 'Map preserves canonical wording; use the normal editorial workflow to edit it')
            entry = entries[eid]
        refs = entry['provenance']['references']
        if source['id'] not in refs:
            refs.append(source['id'])
        mapping = {'source': source['id'], 'external_id': tid, 'relationship': relationship}
        require(not any(m['source'] == source['id'] and m['external_id'] == tid and m != mapping
                        for m in entry['source_mappings']), 'Conflicting existing relationship; review explicitly')
        if mapping not in entry['source_mappings']:
            entry['source_mappings'].append(mapping)
        changes.append({'action': decision['action'], 'canonical_id': eid, 'term_id': tid,
                        'relationship': relationship, 'source_flags': terms[tid]['flags'],
                        'note': decision.get('note', '')})
    warnings = validate(result, manifest)
    report = {'source': source, 'changes': changes, 'alias_collisions': warnings,
              'pending_decisions': len(review['decisions']) - len(approved),
              'entries': len(result['entries']), 'ontology_annotations': metadata,
              'selected_source_terms': [terms[tid] for tid in sorted({d['term_id'] for d in approved})],
              'note': 'Complete seed output only. Run seed-vocabulary --validate-only, then the existing database dry-run before deployment.'}
    write_bundle(args.output, {'ingredients.json': result, 'sources.json': manifest, 'report.json': report})
    return {'entries': report['entries'], 'approved_decisions': len(approved),
            'pending_decisions': report['pending_decisions'], 'alias_collisions': len(warnings)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    for name in ('extract', 'build'):
        command = commands.add_parser(name)
        command.add_argument('--owl', type=Path, required=True)
        command.add_argument('--descriptor', type=Path, default=ROOT / 'data/vocabulary/adapters/foodon.json')
        command.add_argument('--seed', type=Path, default=ROOT / 'data/vocabulary/ingredients.json')
        command.add_argument('--sources', type=Path, default=ROOT / 'data/vocabulary/sources.json')
        command.add_argument('--output', type=Path, required=True, help='Fresh output directory; never overwrites inputs')
        if name == 'extract':
            command.add_argument('--pilot', type=Path, default=ROOT / 'data/vocabulary/coverage/foodon-review.json')
        else:
            command.add_argument('--review', type=Path, required=True)
    args = parser.parse_args()
    try:
        print(json.dumps((extract if args.command == 'extract' else build)(args)))
    except (ValueError, KeyError, TypeError, ET.ParseError, OSError) as error:
        parser.exit(1, f'FoodOn adapter failed: {error}\n')


if __name__ == '__main__':
    main()
