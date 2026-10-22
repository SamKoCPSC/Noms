#!/usr/bin/env python3
"""Triage an extracted FoodOn catalog for editorial review, without approving imports."""
import argparse
import collections
import csv
import hashlib
import json
from pathlib import Path
import re

from foodon import ROOT, load, normalize, valid_text


RULES = {
    'classification_label': 'Classification scheme/code in label; select a culinary concept and review its wording.',
    'ontology_wording': 'Ontology wording such as food product/material; inspect scope before shortening.',
    'animal_feed_branch': 'Named ancestry includes food material for animals; check culinary use individually.',
    'hazard_category': 'Named ancestry includes hazardous food material; an ontology category, not an ingredient name.',
    'generic_category': 'Generic/organizational label; usually unsuitable as an initial suggestion.',
    'preparation_or_storage': 'Preparation/storage variant; distinguish essential ingredient forms from recipe instructions.',
    'formulation_qualifier': 'Composition/formulation qualifier; review whether it defines a useful ingredient.',
    'prepared_dish_word': 'Prepared dish vocabulary; review ingredient use rather than excluding by keyword.',
    'special_diet_word': 'Special-diet/formulation vocabulary; check scope. Baby vegetables are not matched.',
    'catchall_word': 'Other/unspecified/catch-all wording needs scope review.',
    'canonical_text_invalid': 'Proposed label/alias/description violates the canonical text contract.',
    'label_over_80_chars': 'Long label; editorial review threshold, not a canonical validation limit.',
    'missing_definition': 'No English definition; use other evidence before approving identity.',
    'shared_label_or_alias': 'Normalized display name is shared by distinct source IDs; no automatic merge.',
    'existing_seed_name': 'Name overlaps a maintained canonical label/alias; review mapping before creating an ID.',
    'editorial_family': 'Mechanical simplification groups multiple labels; evidence of variants, not equivalence.',
    'manual_finding': 'An explicit source-specific editorial recommendation is recorded.',
}
CLASSIFICATION = re.compile(r'\((?:efsa foodex2|gs1 gpc|us cfr|eurofir|codex|ccpr|ec|efg|eurocode2)\)$', re.I)
PATTERNS = {
    'ontology_wording': r'\b(?:food product|food material|food substance|food component)\b',
    'generic_category': r'^(?:food product|plant food product|food additive|emulsifier|food material by .+|food by .+|food \([^()]+\))$',
    'preparation_or_storage': r'\b(?:irradiated|sterilized|pasteurized|frozen|canned|dehydrated|dried|dry|raw|cooked|boiled|fried|sliced|chopped|ground|powdered|unpeeled|peeled)\b',
    'formulation_qualifier': r'\b(?:without|with|containing|excluding|except|added|low|reduced|enriched|fortified|flavo[u]?red)\b',
    'prepared_dish_word': r'\b(?:pizza|sandwich|soup|salad|dinner|casserole|stew|cake|cookie|burger)\b',
    'special_diet_word': r'\b(?:infant|formula|dietary supplement|supplement|therapeutic|medical|enteral|toddler|dietetic|meal replacement)\b',
    'catchall_word': r'\b(?:unspecified|unknown|not specified|other|various|miscellaneous|unidentified|unclassified)\b',
}


def checksum(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def family_key(label):
    # Deliberately lossy diagnostic. Never use this string as canonical identity.
    label = re.sub(r'\([^()]*\)', '', label)
    label = re.sub(r'\bfood product\b', '', label, flags=re.I)
    label = re.sub(r'^\d+\s+(?:-\s+)?', '', label)
    return normalize(label)


def named_descendants(terms, root):
    children = collections.defaultdict(set)
    for term in terms:
        for parent in term['parents']:
            children[parent].add(term['id'])
    seen, pending = set(), [root]
    while pending:
        tid = pending.pop()
        if tid not in seen:
            seen.add(tid)
            pending.extend(children[tid] - seen)
    return seen


def cell(value):
    value = str(value).replace('|', '\\|').replace('\n', ' ')
    return ''.join(f'\\u{ord(c):04x}' if ord(c) < 32 or 127 <= ord(c) <= 159 else c for c in value)


def audit(catalog, seed, review):
    if catalog['schema_version'] != 1 or catalog['source']['checksum'] != review['source_checksum']:
        raise ValueError('Review/catalog source identity differs')
    all_terms = {r['id']: r for r in catalog['terms']}
    if len(all_terms) != len(catalog['terms']):
        raise ValueError('Duplicate catalog term IDs')
    terms = sorted((r for r in all_terms.values() if r['eligible']), key=lambda r: r['id'])
    notes = {}
    for finding in review['findings']:
        term = all_terms[finding['term_id']]
        if not term['eligible'] or term['label'] != finding['source_label']:
            raise ValueError('Stale editorial finding: ' + finding['term_id'])
        if finding['term_id'] in notes:
            raise ValueError('Repeated editorial finding')
        notes[finding['term_id']] = finding
    names, families, seed_names = collections.defaultdict(dict), collections.defaultdict(list), collections.defaultdict(set)
    for entry in seed['entries']:
        for name in [entry['label'], *entry['aliases']]:
            seed_names[normalize(name)].add(entry['id'])
    for term in terms:
        for name in [term['label'], *term['canonical_proposal']['aliases']]:
            names[normalize(name)][term['id']] = name
        families[family_key(term['label'])].append(term['id'])
    collisions = {name: group for name, group in names.items() if len(group) > 1}
    shared = collections.defaultdict(list)
    for name, group in collisions.items():
        for tid in group:
            shared[tid].append(name)
    feeds = named_descendants(catalog['terms'], 'FOODON_03309997')
    hazards = named_descendants(catalog['terms'], 'FOODON_00005146')
    rows = []
    for term in terms:
        label, tid, proposal = term['label'], term['id'], term['canonical_proposal']
        flags = [name for name, pattern in PATTERNS.items() if re.search(pattern, label, re.I)]
        if CLASSIFICATION.search(label):
            flags.append('classification_label')
        for condition, flag in (
            (tid in feeds, 'animal_feed_branch'), (tid in hazards, 'hazard_category'),
            (len(label) > 80, 'label_over_80_chars'), (not term['definitions'], 'missing_definition'),
            (not valid_text(label) or len(proposal['description']) > 2000 or
             len(proposal['aliases']) > 100 or any(not valid_text(a) for a in proposal['aliases']), 'canonical_text_invalid'),
            (bool(shared.get(tid)), 'shared_label_or_alias'), (tid in notes, 'manual_finding'),
            (len(families[family_key(label)]) > 1, 'editorial_family'),
        ):
            if condition:
                flags.append(flag)
        overlap = sorted({eid for name in [label, *proposal['aliases']] for eid in seed_names[normalize(name)]})
        if overlap:
            flags.append('existing_seed_name')
        finding = notes.get(tid, {})
        rows.append(dict(term_id=tid, label=label, flags=sorted(flags),
                         aliases=proposal['aliases'], definitions=term['definitions'],
                         shared_names=sorted(shared.get(tid, [])), existing_canonical_ids=overlap,
                         family=family_key(label), family_size=len(families[family_key(label)]),
                         recommendation=finding.get('recommendation', ''), note=finding.get('note', ''),
                         proposed_label=finding.get('proposed_label', '')))
    counts = collections.Counter(flag for row in rows for flag in row['flags'])
    # Missing definitions and preparation/family flags alone do not prove poor wording.
    attention_flags = set(RULES) - {'missing_definition', 'preparation_or_storage', 'editorial_family'}
    summary = dict(total_terms=len(all_terms), eligible_terms=len(terms),
                   flagged_for_editorial_attention=sum(bool(set(r['flags']) & attention_flags) for r in rows),
                   rule_counts={rule: counts[rule] for rule in RULES},
                   exact_normalized_label_duplicate_groups=sum(
                       len(ids) > 1 for ids in _label_groups(terms).values()),
                   shared_name_groups=len(collisions), shared_name_terms=len(shared),
                   editorial_families=sum(len(ids) > 1 for ids in families.values()),
                   editorial_family_terms=sum(len(ids) for ids in families.values() if len(ids) > 1),
                   manual_findings=len(notes), approved_imports=0)
    return rows, collisions, summary


def _label_groups(terms):
    groups = collections.defaultdict(set)
    for term in terms:
        groups[normalize(term['label'])].add(term['id'])
    return groups


def report(summary, review, source):
    lines = ['# FoodOn candidate label review', '',
             'Reviewed snapshot: ' + source['version'] + '.', '',
             f"The extracted catalog has **{summary['total_terms']:,} classes**, of which **{summary['eligible_terms']:,}** pass the adapter's structural scope checks. Structural eligibility is not editorial approval.", '',
             'This is a full-catalog mechanical triage plus selected semantic/editorial findings. It does not claim an individual semantic review of every term. Flags overlap and do not count accepted/rejected ingredients. No seed files, UUIDs, mapping approvals or database rows are changed.', '',
             '| Review signal | Eligible terms | Interpretation |', '|---|---:|---|']
    for key, description in RULES.items():
        lines.append(f"| `{key}` | {summary['rule_counts'][key]:,} | {description} |")
    lines += ['', f"There are **{summary['shared_name_groups']:,} shared normalized label/alias strings** across **{summary['shared_name_terms']:,} terms**, despite **{summary['exact_normalized_label_duplicate_groups']} exact normalized label-duplicate groups**. Including aliases is essential.", '',
              f"Removing parentheses, numeric prefixes and 'food product' as a diagnostic groups **{summary['editorial_family_terms']:,} terms into {summary['editorial_families']:,} families**. Those families are deliberately lossy: raw/canned/dried, active/inactive, species and edible-part distinctions must survive when meaningful. Never use this simplification to merge identities.", '',
              f"**{summary['flagged_for_editorial_attention']:,}** eligible terms have at least one attention flag other than preparation/family membership or missing definition alone. The remainder is not an approved clean set.", '',
              '## Selected findings and recommendations', '',
              f"The {summary['manual_findings']} findings below are editorial proposals, not approved adapter import decisions. Suggested replacements require identity/scope review before ingestion.", '',
              '| Source ID | Source label | Recommendation | Reason / proposed wording |', '|---|---|---|---|']
    for finding in review['findings']:
        text = finding['note']
        if finding.get('proposed_label'):
            text += ' Proposed label: ' + finding['proposed_label'] + '.'
        lines.append('| ' + ' | '.join(map(cell, [finding['term_id'], finding['source_label'], finding['recommendation'], text])) + ' |')
    lines += ['', '## Proposed initial-seed policy', '',
              '- Keep specific culinary ingredients, including unfamiliar regional ingredients; unfamiliarity is not an exclusion reason.',
              '- Hold organizational/regulatory categories, animal-specific products and hazard/state categories out of the initial suggestion list after checking actual scope. Feed ancestry alone is not sufficient: bee pollen also occurs there.',
              '- Prefer reviewed culinary labels. Keep source codes/labels in provenance and mapping evidence. Removing a suffix does not establish exact equivalence.',
              '- Resolve shared aliases and existing-seed matches before assigning a new canonical UUID. Do not merge peppercorns with Capsicum, allspice with pimiento, or caper buds with caper fruit.',
              '- Preserve forms that change the ingredient: dried versus fresh yeast, active versus inactive yeast, dried versus canned tomatoes, powders/pastes/oils versus their source materials. Review incidental slicing/packaging/storage separately.',
              '- Decide prepared-food scope by recipe use: bread and canned mushroom soup can be ingredients; named complete dishes are lower priority. Do not exclude all soup/salad/baby keywords.',
              '- Repair invalid text and quarantine mistaken source synonyms. FoodOn exact-synonym annotations still require editorial checking.',
              '- Preserve meaningful numbers and punctuation in names. For example, 4-hexylresorcinol is a chemical name, not a classification code; the adapter\'s broad technical-label warning is a review hint, not an exclusion rule.',
              '- Preserve the existing pilot\'s broader/narrower/related judgments. Cosmetic label cleanup does not repair mismatched definitions or license an exact mapping.', '',
              '## Reproduce', '', '```bash',
              'python3 scripts/vocabulary/audit_foodon_labels.py \\',
              '  --catalog /path/to/extracted/candidates.json \\',
              '  --output /tmp/noms-foodon-label-audit', '```', '',
              'The fresh output directory contains all eligible candidates in `candidates.csv`, shared strings in `shared-names.csv`, `summary.json`, input hashes in `inputs.json`, and this report. Raw catalogs and full candidate dumps stay outside Git. The source-specific findings are maintained in `data/vocabulary/adapters/foodon-label-review.json`.', '']
    return '\n'.join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--catalog', type=Path, required=True)
    parser.add_argument('--seed', type=Path, default=ROOT / 'data/vocabulary/adapters/foodon-cooking-base-ingredients.json',
                        help='Frozen pre-expansion benchmark by default; override to audit against another seed')
    parser.add_argument('--review', type=Path, default=ROOT / 'data/vocabulary/adapters/foodon-label-review.json')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error('Output directory already exists; choose a fresh path')
    catalog, seed, review = load(args.catalog), load(args.seed), load(args.review)
    rows, collisions, summary = audit(catalog, seed, review)
    args.output.mkdir(parents=True)
    with (args.output / 'candidates.csv').open('w', newline='', encoding='utf-8') as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
        writer.writeheader()
        for row in rows:
            writer.writerow({k: json.dumps(v, ensure_ascii=False) if isinstance(v, list) else v for k, v in row.items()})
    with (args.output / 'shared-names.csv').open('w', newline='', encoding='utf-8') as stream:
        writer = csv.writer(stream)
        writer.writerow(['normalized_name', 'term_ids'])
        for name, group in sorted(collisions.items()):
            writer.writerow([name, json.dumps(sorted(group))])
    (args.output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    (args.output / 'inputs.json').write_text(json.dumps({key: dict(sha256=checksum(path))
        for key, path in [('catalog', args.catalog), ('seed', args.seed), ('review', args.review)]}, indent=2) + '\n')
    (args.output / 'README.md').write_text(report(summary, review, catalog['source']), encoding='utf-8')
    print(json.dumps(summary, indent=2))


if __name__ == '__main__':
    main()
