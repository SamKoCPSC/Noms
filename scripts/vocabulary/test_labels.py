"""Identity, compatibility and replay checks for the preferred-label review."""
import copy
import json
import unittest

import rename_labels as labels
from fill_gaps import checksum
from foodon import ROOT, normalize, validate


class PreferredLabelTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.policy = json.loads(labels.DEFAULT_POLICY.read_text())
        cls.seed = labels.maintained_label_stage(json.loads((ROOT / 'data/vocabulary/ingredients.json').read_text()))
        cls.sources = json.loads((ROOT / 'data/vocabulary/sources.json').read_text())
        cls.base = labels.restore_baseline(cls.seed, cls.policy)
        cls.entries = {e['id']: e for e in cls.seed['entries']}
        cls.oldlabels = {e['label']: e['id'] for e in cls.base['entries']}

    def test_replay_and_historical_baseline_are_exact(self):
        self.assertEqual(checksum(self.base), '373d0b7b4c19f6c8e664655d1ebafedbae97e8028503cca63733c62e9609dd80')
        self.assertEqual(labels.apply_review(self.base, self.sources, self.policy), self.seed)
        self.assertEqual(validate(self.seed, self.sources), [])
        self.assertEqual(len(self.entries), 13136)

    def test_every_previous_name_and_identity_survives(self):
        for before in self.base['entries']:
            after = self.entries[before['id']]
            self.assertEqual({k: v for k, v in before.items() if k not in {'label', 'aliases'}},
                             {k: v for k, v in after.items() if k not in {'label', 'aliases'}})
            self.assertLessEqual(set([before['label'], *before['aliases']]),
                                 set([after['label'], *after['aliases']]))
            if before['status'] == 'retired' or '(' not in before['label']:
                self.assertEqual(before, after)
        self.assertEqual(sum(e['status'] == 'retired' for e in self.seed['entries']), 81)

    def test_cooking_states_parts_fat_and_pack_medium_stay_explicit(self):
        expected = {
            'tomato (canned)': 'canned tomatoes',
            'chicken breast (skinless, boneless)': 'boneless skinless chicken breast',
            'lamb meat (with bone)': 'bone-in lamb meat',
            'almond (shell off)': 'shelled almonds',
            'barley (pearled)': 'pearl barley',
            'beef (ground, extra lean)': 'extra lean ground beef',
            'tomato (pieces, canned)': 'canned tomato pieces',
            'light whipping cream (30–36% milk fat)': 'light whipping cream with 30–36% milk fat',
            'tuna (solid pack, canned in olive oil)': 'solid-pack tuna canned in olive oil',
        }
        for before, after in expected.items():
            entry = self.entries[self.oldlabels[before]]
            self.assertEqual(entry['label'], after)
            self.assertIn(before, entry['aliases'])
        self.assertNotEqual(self.oldlabels['tomato (canned)'], self.oldlabels['tomato (dried)'])

    def test_scientific_qualifiers_and_colliding_names_are_retained(self):
        for name in ['butterbur (Petasites japonicus)', 'canarium nuts (harveyi)',
                     'canarium nuts (indicum)', 'salmon (canned)', 'tomato (sun-dried)']:
            ident = self.oldlabels[name]
            self.assertIn(ident, self.policy['holds'])
            self.assertEqual(self.entries[ident]['label'], name)
        for ident, decision in self.policy['holds'].items():
            if decision['reason'].startswith('name_owned_by_another_identity: '):
                for other in decision['reason'].split(': ', 1)[1].split(','):
                    self.assertNotEqual(ident, other)
                    self.assertEqual(self.entries[other]['status'], 'active')

    def test_stale_pins_incomplete_review_and_alias_promotion_fail_closed(self):
        with self.assertRaisesRegex(ValueError, 'Stale'):
            labels.apply_review(self.seed, self.sources, self.policy)
        p = copy.deepcopy(self.policy);p['approved'] = False
        with self.assertRaisesRegex(ValueError, 'Unapproved'):
            labels.apply_review(self.base, self.sources, p)
        p = copy.deepcopy(self.policy);p['holds'].pop(next(iter(p['holds'])))
        with self.assertRaisesRegex(ValueError, 'exactly one'):
            labels.apply_review(self.base, self.sources, p)
        p = copy.deepcopy(self.policy)
        decision = next(d for d in p['changes'].values() if 'promoted_alias_index' in d)
        decision['promoted_alias_index'] += 1
        with self.assertRaisesRegex(ValueError, 'promoted alias'):
            labels.apply_review(self.base, self.sources, p)

    def test_another_identity_cannot_be_claimed_by_a_preferred_label(self):
        p = copy.deepcopy(self.policy)
        decision = p['changes'][self.oldlabels['tomato (canned)']]
        decision['after'] = 'chickpea';decision.pop('promoted_alias_index', None)
        with self.assertRaisesRegex(ValueError, 'collide'):
            labels.apply_review(self.base, self.sources, p)
        tampered = copy.deepcopy(self.seed)
        tampered['entries'][0]['description'] += 'changed'
        with self.assertRaisesRegex(ValueError, 'approved label output'):
            labels.restore_baseline(tampered, self.policy)


if __name__ == '__main__':
    unittest.main()
