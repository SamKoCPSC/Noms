#!/usr/bin/env python3
"""Screen every captured entity and maintained name under an explicit cooking policy.

Policy-screened name overlaps may supply related evidence, never exact identity.
New imports and stronger relationships require explicit source-specific decisions.
"""
import argparse
import collections
import csv
import json
from pathlib import Path
import re
import uuid

from foodon import ROOT, digest, load, normalize, require, write_bundle
import wikidata as wd


def disposition(term, policy):
    flags = set(term['flags'])
    if flags - {'missing_english_description', 'outside_ingredient_discovery'}:
        return 'hold_source_flags'
    if 'outside_ingredient_discovery' in flags:
        return 'hold_outside_discovery'
    for rule in policy['hold_rules']:
        text = term['label'] if rule['field'] == 'label' else term['label'] + ' ' + term['description']
        if re.search(rule['pattern'], text, re.I):
            return rule['status']
    if not term['description']:
        return 'hold_missing_description'
    return 'review_culinary_candidate'


def prepare(args):
    descriptor, seed, sources, terms = wd.context(args)
    policy = load(args.policy)
    require(policy['schema_version'] == 1 and policy['descriptor_checksum'] == digest(args.descriptor),
            'Policy targets a different Wikidata descriptor')
    index = wd.name_index(seed)
    by_id = {e['id']: e for e in seed['entries']}
    registry = load(args.registry) if args.registry.exists() else {'decisions': []}
    prior = {d['term_id']: d['canonical_id'] for d in registry['decisions'] if d['action'] == 'import'}
    explicit = policy['decisions']
    require(len({d['term_id'] for d in explicit}) == len(explicit), 'Repeated explicit entity decision')
    require(all(d['term_id'] in terms for d in explicit), 'Policy references an uncaptured entity')
    overrides = {d['term_id']: d for d in explicit}
    rows, decisions, matched = [], [], collections.defaultdict(list)
    ownership = {k: set(v) for k, v in index.items()}
    # Allocate every approved import before resolving mappings. A related source
    # record may target a newly approved identity regardless of QID sort order.
    import_ids = {}
    for d in explicit:
        if d['action'] != 'import':
            continue
        qid = d['term_id']
        eid = prior.get(qid) or str(uuid.uuid4())
        import_ids[qid] = eid
        by_id[eid] = dict(id=eid, **d['canonical'], status='active')
        for name in [d['canonical']['label'], *d['canonical']['aliases']]:
            key = normalize(name)
            require(not ownership.get(key), f'Explicit import collides with maintained name: {name}')
            ownership[key] = {eid}
    for qid, term in terms.items():
        status = disposition(term, policy)
        matches = sorted(set().union(*(index[normalize(n)] for n in [term['label'], *term['aliases']] if n)))
        row = dict(term_id=qid, revision=term['revision'], label=term['label'], description=term['description'],
                   status=status, name_matches=matches, flags=term['flags'])
        override = overrides.get(qid)
        if override:
            action = override['action']
            if action == 'hold':
                row.update(status='hold_editorial', note=override['note'])
            else:
                require(action in {'map', 'import'}, 'Invalid explicit review action')
                d = dict(approved=True, action=action, term_id=qid, revision=term['revision'],
                         relationship=override['relationship'], note=policy['version'] + '; ' + override['note'])
                if action == 'map':
                    target = override.get('target_term_id')
                    require(target is None or target in import_ids, 'Map target must be a reviewed import')
                    eid = import_ids[target] if target else override['canonical_id']
                    require(eid in by_id and by_id[eid]['status'] == 'active', 'Explicit map needs an active canonical ID')
                else:
                    eid = import_ids[qid]
                    d['canonical'] = override['canonical']
                d['canonical_id'] = eid
                decisions.append(d)
                matched[eid].append(qid)
                row.update(status='approved_' + action, canonical_id=eid, note=d['note'])
        elif status == 'review_culinary_candidate':
            label_matches = sorted(index[normalize(term['label'])])
            if len(matches) == 1 and label_matches == matches:
                # The policy authorizes association only. Cosmetic equality does not prove equivalence.
                eid = matches[0]
                decisions.append(dict(approved=True, action='map', term_id=qid, revision=term['revision'],
                    canonical_id=eid, relationship='related',
                    note=policy['version'] + '; ingredient-scope entity with an unambiguous normalized label overlap. Related discovery evidence only; no exact-equivalence claim or alias adoption.'))
                matched[eid].append(qid)
                row.update(status='approved_related_name', canonical_id=eid, note=decisions[-1]['note'])
            elif matches:
                row['status'] = 'hold_alias_or_identity_ambiguity'
            else:
                row['status'] = 'hold_new_identity_review'
        rows.append(row)
    # Never silently discard persisted canonical identities on an editorial rerun.
    require(set(prior) <= {d['term_id'] for d in decisions if d['action'] == 'import'},
            'Policy drops a persisted import; retain it explicitly and review retirement through the canonical workflow')
    # Imports precede maps to those identities in the canonical builder.
    decisions.sort(key=lambda d: d['action'] != 'import')
    review = {**wd.bindings(args, descriptor), 'decisions': decisions}
    coverage = [dict(canonical_id=e['id'], label=e['label'], aliases=e['aliases'],
                     mappings=sorted(matched[e['id']]), status='mapped' if matched[e['id']] else 'held_or_unmatched')
                for e in seed['entries'] if e['status'] == 'active']
    summary = dict(entities=len(rows), source_dispositions=dict(sorted(collections.Counter(r['status'] for r in rows).items())),
                   active_seed_names=len(coverage), seed_names_mapped=sum(bool(r['mappings']) for r in coverage),
                   approved_decisions=len(decisions), imports=sum(d['action'] == 'import' for d in decisions),
                   policy_checksum=digest(args.policy),
                   method='All captured entities and active seed names mechanically screened; explicit editorial overrides. Holds and unmatched names are not absence claims or semantic certification.')
    write_bundle(args.output, {'review.json': review, 'source-name-review.json': rows,
                              'seed-name-review.json': coverage, 'summary.json': summary})
    for name, records, fields in [('source-names.csv', rows, ['term_id','revision','label','description','status','name_matches','flags','canonical_id','note']),
                                  ('seed-names.csv', coverage, ['canonical_id','label','aliases','status','mappings'])]:
        with (args.output/name).open('w', encoding='utf-8-sig', newline='') as stream:
            writer=csv.DictWriter(stream,fieldnames=fields,extrasaction='ignore');writer.writeheader()
            for row in sorted(records,key=lambda r: normalize(r['label'])):
                writer.writerow({k: ' | '.join(v) if isinstance(v,list) else v for k,v in row.items() if k in fields})
    return summary


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--snapshot',type=Path,required=True)
    parser.add_argument('--descriptor',type=Path,default=ROOT/'data/vocabulary/adapters/wikidata.json')
    parser.add_argument('--seed',type=Path,default=ROOT/'data/vocabulary/ingredients.json')
    parser.add_argument('--sources',type=Path,default=ROOT/'data/vocabulary/sources.json')
    parser.add_argument('--policy',type=Path,default=ROOT/'data/vocabulary/adapters/wikidata-cooking-policy.json')
    parser.add_argument('--registry',type=Path,default=ROOT/'data/vocabulary/adapters/wikidata-cooking-review.json')
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args();print(json.dumps(prepare(args)))


if __name__=='__main__':main()
