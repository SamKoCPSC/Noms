"""Replay the checksum-pinned preferred-label review, preserving ingredient identity.

This is an explicit per-UUID ledger, not a rule applied to future source imports.
Old preferred labels remain aliases. An existing alias may become the preferred
label without duplicating its normalized name. No recipes or source data change.
"""
import argparse
import copy
import csv
import json
from pathlib import Path

from fill_gaps import checksum, encoded, SECOND_SAMPLE_POLICY, GROCERY_POLICY, FINAL_GAP_POLICY, NEXT_GAP_POLICY, restore_baseline as restore_gap_baseline
from foodon import ROOT, normalize, require, validate

DEFAULT_POLICY = ROOT / 'data/vocabulary/adapters/ingredient-label-review.json'


def apply_review(seed, sources, policy):
    require(policy['schema_version'] == 1 and policy['approved'] is True, 'Unapproved label review')
    require(checksum(seed) == policy['base_seed_checksum'] and
            checksum(sources) == policy['base_sources_checksum'], 'Stale label-review baseline')
    require(not validate(seed, sources), 'Invalid label-review baseline')
    changes, holds = policy['changes'], policy['holds']
    expected = {e['id'] for e in seed['entries'] if e['status'] == 'active' and '(' in e['label']}
    require(not set(changes) & set(holds) and set(changes) | set(holds) == expected,
            'Every active parenthetical label needs exactly one review decision')
    result = copy.deepcopy(seed)
    for entry in result['entries']:
        ident = entry['id']
        if ident in holds:
            require(holds[ident]['label'] == entry['label'] and holds[ident]['reason'].strip(),
                    'Stale or unexplained label hold')
        if ident not in changes:
            continue
        decision = changes[ident]
        require(entry['label'] == decision['before'], 'Stale preferred label')
        new_label = decision['after']
        require(new_label.strip() == new_label and new_label and '(' not in new_label and
                normalize(new_label) != normalize(entry['label']), 'Invalid preferred-label replacement')
        matches = [i for i, n in enumerate(entry['aliases']) if normalize(n) == normalize(new_label)]
        promoted = decision.get('promoted_alias_index')
        require(matches == ([] if promoted is None else [promoted]), 'Stale promoted alias')
        if promoted is not None:
            require(type(promoted) is int and entry['aliases'][promoted] == new_label,
                    'Promoted alias wording changed')
            entry['aliases'].pop(promoted)
        entry['aliases'].append(entry['label'])
        entry['label'] = new_label
    require(not validate(result, sources), 'Renamed labels collide or violate the canonical schema')
    for old, new in zip(seed['entries'], result['entries']):
        require({k: v for k, v in old.items() if k not in {'label', 'aliases'}} ==
                {k: v for k, v in new.items() if k not in {'label', 'aliases'}},
                'Label review changed ingredient identity or evidence')
        require(set([old['label'], *old['aliases']]) <= set([new['label'], *new['aliases']]),
                'Label review removed a prior searchable name')
        require(old['status'] == 'active' or old == new, 'Retired ingredient changed')
    require(checksum(result) == policy['expected_output_checksum'], 'Output differs from approved label checksum')
    return result


def restore_baseline(result, policy=None):
    """Recover the exact preceding curation stage for historical stage checks."""
    if policy is None:
        policy = json.loads(DEFAULT_POLICY.read_text())
    require(checksum(result) == policy['expected_output_checksum'], 'Not the approved label output')
    base = copy.deepcopy(result)
    for entry in base['entries']:
        decision = policy['changes'].get(entry['id'])
        if decision is None:
            continue
        require(entry['label'] == decision['after'] and entry['aliases'][-1] == decision['before'],
                'Renamed label or retained old alias changed')
        entry['aliases'].pop()
        if 'promoted_alias_index' in decision:
            entry['aliases'].insert(decision['promoted_alias_index'], entry['label'])
        entry['label'] = decision['before']
    require(checksum(base) == policy['base_seed_checksum'], 'Restored label baseline differs from approved input')
    return base


def maintained_final_gap_stage(result):
    """Verify/remove the next-sample grocery expansion without changing old pins."""
    return restore_gap_baseline(result, json.loads(NEXT_GAP_POLICY.read_text()))


def maintained_grocery_stage(result):
    """Verify/remove the final gap-fill and explicit retirement layer."""
    return restore_gap_baseline(maintained_final_gap_stage(result), json.loads(FINAL_GAP_POLICY.read_text()))


def maintained_second_sample_stage(result):
    """Recover the second-sample stage through later pinned curation layers."""
    return restore_gap_baseline(maintained_grocery_stage(result), json.loads(GROCERY_POLICY.read_text()))


def maintained_label_stage(result):
    """Recover the label stage through the explicitly pinned later gap layers."""
    return restore_gap_baseline(maintained_second_sample_stage(result), json.loads(SECOND_SAMPLE_POLICY.read_text()))


def restore_maintained_baseline(result):
    """Recover the pre-label seed from the current maintained curation pipeline."""
    return restore_baseline(maintained_label_stage(result))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--seed', type=Path, required=True)
    parser.add_argument('--sources', type=Path, required=True)
    parser.add_argument('--policy', type=Path, default=DEFAULT_POLICY)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    policy = json.loads(args.policy.read_text())
    sources = json.loads(args.sources.read_text())
    result = apply_review(json.loads(args.seed.read_text()), sources, policy)
    require(not args.output.exists(), 'Use a fresh output directory')
    args.output.mkdir(parents=True)
    (args.output / 'ingredients.json').write_bytes(encoded(result))
    (args.output / 'sources.json').write_bytes(encoded(sources))
    with (args.output / 'label-review.csv').open('w', newline='') as stream:
        writer = csv.DictWriter(stream, fieldnames=['id', 'before', 'after', 'action', 'reason'])
        writer.writeheader()
        for ident, decision in policy['changes'].items():
            writer.writerow(dict(id=ident, before=decision['before'], after=decision['after'],
                                 action='rename', reason='Reviewed natural cooking name; previous names retained'))
        for ident, decision in policy['holds'].items():
            writer.writerow(dict(id=ident, before=decision['label'], after=decision['label'],
                                 action='retain', reason=decision['reason']))
    print(json.dumps(dict(renamed=len(policy['changes']), retained=len(policy['holds']),
                          active=sum(e['status'] == 'active' for e in result['entries']),
                          active_aliases=sum(len(e['aliases']) for e in result['entries'] if e['status'] == 'active'),
                          output_checksum=checksum(result)), indent=2))


if __name__ == '__main__':
    main()
