"""Research boundary checks: provenance, scope, identity ambiguity and replay."""
import argparse
import copy
import csv
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import equipment_research as research
from foodon import digest


def entity(qid, label, aliases=()):
    return dict(id=qid, type='item', lastrevid=int(qid[1:]),
        labels={'en': {'language': 'en', 'value': label}},
        descriptions={'en': {'language': 'en', 'value': 'Kitchen tool'}},
        aliases={'en': [{'language': 'en', 'value': value} for value in aliases]}, claims={})


class EquipmentResearchTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        root = Path(self.temp.name)
        self.args = argparse.Namespace(**{key: root/key for key in ['snapshot', 'descriptor', 'policy', 'benchmark', 'output']})
        self.entities = [entity('Q1', 'cake pan', ['cake tin']), entity('Q2', 'cake pan', ['baking pan'])]
        self.args.snapshot.write_text(json.dumps(dict(schema_version=1, entities=self.entities)))
        checksum = digest(self.args.snapshot)
        self.descriptor = dict(schema_version=1, bytes=self.args.snapshot.stat().st_size,
            source=dict(checksum=checksum, license='CC0-1.0'), raw_records=3, revisions={'Q1': 1, 'Q2': 2})
        self.policy = dict(schema_version=1, source_checksum=checksum, choices={
            'Q1': dict(label='cake pan', family='bakeware', tier='core', reason='Proposed for recipe equipment review.')})
        self.write('descriptor', self.descriptor)
        self.write('policy', self.policy)
        self.write('benchmark', [dict(label='cake tin', reference='fixture'), dict(label='baking pan', reference='fixture'),
                                 dict(label='cake', reference='fixture')])

    def write(self, key, data):
        getattr(self.args, key).write_text(json.dumps(data))

    def evaluate(self):
        with patch('urllib.request.urlopen', side_effect=AssertionError('Evaluation must be offline')), patch('builtins.print'):
            research.evaluate(self.args)
        return json.loads((self.args.output/'summary.json').read_text())

    def csv_rows(self, name):
        with (self.args.output/name).open(encoding='utf-8-sig') as stream:
            return list(csv.DictReader(stream))

    def test_every_source_identity_survives_and_name_overlap_cannot_approve_it(self):
        summary = self.evaluate()
        self.assertEqual(summary['detailed_records'], 2)
        self.assertEqual(summary['duplicate_primary_name_groups'], 1)
        self.assertEqual(summary['dispositions']['recommended_candidate'], 1)
        rows = self.csv_rows('all-candidates.csv')
        self.assertEqual({row['source_id'] for row in rows}, {'Q1', 'Q2'})
        coverage = self.csv_rows('coverage.csv')
        self.assertEqual([row['status'] for row in coverage],
                         ['shortlisted_name_match', 'unreviewed_or_excluded_name_match', 'no_name_match_in_capture'])

    def test_snapshot_drift_and_unknown_or_duplicate_review_ids_fail_closed(self):
        self.args.snapshot.write_text(self.args.snapshot.read_text()+' ')
        with self.assertRaisesRegex(ValueError, 'checksum'):
            self.evaluate()
        self.args.snapshot.write_text(self.args.snapshot.read_text()[:-1])
        self.policy['choices']['Q9'] = copy.deepcopy(self.policy['choices']['Q1'])
        self.write('policy', self.policy)
        with self.assertRaisesRegex(ValueError, 'uncaptured'):
            self.evaluate()
        del self.policy['choices']['Q9']
        self.policy['choices']['Q2'] = copy.deepcopy(self.policy['choices']['Q1'])
        self.write('policy', self.policy)
        with self.assertRaisesRegex(ValueError, 'duplicate normalized'):
            self.evaluate()

    def test_editorial_names_are_distinguished_from_upstream_names_and_replay_is_deterministic(self):
        self.policy['choices']['Q1']['label'] = 'round cake pan'
        self.write('policy', self.policy)
        self.write('benchmark', [dict(label='round cake pan', reference='fixture')])
        before = self.args.snapshot.read_bytes()
        self.evaluate()
        rows = self.csv_rows('coverage.csv')
        self.assertEqual(rows[0]['match_basis'], 'proposed_label_only')
        first = {path.name: path.read_bytes() for path in self.args.output.iterdir()}
        self.evaluate()
        self.assertEqual(first, {path.name: path.read_bytes() for path in self.args.output.iterdir()})
        self.assertEqual(before, self.args.snapshot.read_bytes())

    def test_missing_english_labels_cannot_be_recommended_by_policy(self):
        item = entity('Q3', '')
        with self.assertRaisesRegex(ValueError, 'Ineligible'):
            research.disposition(item, dict(reason='Not enough evidence'))
        self.assertEqual(research.disposition(item)[0], 'hold_for_review')


if __name__ == '__main__':
    unittest.main()
