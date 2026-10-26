"""Content/identity regression checks for the maintained cooking seed."""
import json
from pathlib import Path
import unittest

from foodon import ROOT, normalize, validate


class CookingSeedTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.seed = json.loads((ROOT / 'data/vocabulary/ingredients.json').read_text())
        cls.manifest = json.loads((ROOT / 'data/vocabulary/sources.json').read_text())
        cls.base = json.loads((ROOT / 'data/vocabulary/adapters/foodon-cooking-base-ingredients.json').read_text())
        cls.review = json.loads((ROOT / 'data/vocabulary/adapters/foodon-cooking-review.json').read_text())
        cls.by_id = {e['id']: e for e in cls.seed['entries']}
        cls.by_name = {normalize(e['label']): e for e in cls.seed['entries'] if e['status'] == 'active'}

    def test_complete_seed_retains_original_authored_identities(self):
        self.assertEqual(validate(self.seed, self.manifest), [])
        for entry in self.base['entries']:
            current = self.by_id[entry['id']]
            for key in ['label', 'aliases', 'description', 'status', 'locale']:
                self.assertEqual(current[key], entry[key], (entry['label'], key))
            self.assertEqual(current['provenance']['origin'], 'curated')
        cumin = self.by_id['1745729e-d12c-454d-92cd-1c448f08dd23']
        self.assertEqual((cumin['label'], cumin['aliases']), ('cumin seed', ['cumin']))

    def test_display_names_are_unique_and_do_not_expose_source_categories(self):
        active = [e for e in self.seed['entries'] if e['status'] == 'active']
        self.assertEqual(len(self.by_name), len(active))
        for entry in active:
            self.assertNotRegex(entry['label'].lower(), r'\b(?:efsa foodex2|gs1 gpc|us cfr|food product|food material|animal feed|pet food|infant formula)\b')
        for label in ['food product', 'food (spoiled)', 'hazardous food material',
                      'pizza', 'hawaiian pizza', 'Glucuronolactone', 'basic methacrylate copolymer',
                      'microbial refined', 'composite dishes', 'rice based dishes, cooked',
                      'solution for injection', 'parenteral preparation', 'liquid, higher viscosity']:
            self.assertNotIn(normalize(label), self.by_name)

    def test_retirement_preserves_staged_identities_but_hides_categories(self):
        retired = [e for e in self.seed['entries'] if e['status'] == 'retired']
        self.assertTrue(retired)
        for entry in retired:
            self.assertIn(entry['id'], self.by_id)
            self.assertNotIn(normalize(entry['label']), self.by_name)
        self.assertEqual(len(self.by_id), len(self.seed['entries']))

    def test_useful_forms_and_pantry_exceptions_survive_filtering(self):
        for label in ['active dry yeast', 'inactive dry yeast', 'fresh yeast',
                      'baby spinach', 'bee pollen', 'canned mushroom soup',
                      'cream of mushroom soup', 'baking soda', 'baking powder',
                      'all-purpose flour', 'cream of tartar', 'vanilla extract']:
            self.assertIn(label, self.by_name)
        ids = {self.by_name[name]['id'] for name in ['active dry yeast', 'inactive dry yeast', 'fresh yeast']}
        self.assertEqual(len(ids), 3)
        self.assertNotEqual(self.by_name['tomato']['id'], self.by_name['tomato (sun-dried)']['id'])

    def test_persisted_decisions_match_seed_and_retain_pilot_relationships(self):
        for decision in self.review['decisions']:
            entry = self.by_id[decision['canonical_id']]
            self.assertIn(dict(source='foodon', external_id=decision['term_id'],
                               relationship=decision['relationship']), entry['source_mappings'])
            if decision['action'] == 'import':
                for key, value in decision['canonical'].items():
                    self.assertEqual(entry[key], value)
        pilot = json.loads((ROOT / 'data/vocabulary/adapters/foodon-pilot-review.json').read_text())
        for decision in pilot['decisions']:
            self.assertIn(decision, self.review['decisions'])
        # A good display label does not change these imperfect source definitions into exact matches.
        for label in ['saffron', 'taro']:
            self.assertTrue(any(m['relationship'] == 'related' for m in self.by_name[label]['source_mappings']))


if __name__ == '__main__':
    unittest.main()
