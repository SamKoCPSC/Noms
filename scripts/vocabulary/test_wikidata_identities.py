"""Identity-review regressions: preserve culinary scope and published UUIDs."""
import collections
import copy
import json
import unittest

from foodon import digest, load
import resolve_wikidata_identities as identities
from review_wikidata import prepare
import test_wikidata as fixtures
from test_wikidata import EXISTING, RETIRED, entity
import wikidata as wd


def term(label, description, **values):
    return dict(id='Q1', label=label, description=description, aliases=[],
                instance_of=[], subclass_of=[], **values)


class ScopeTest(unittest.TestCase):
    def test_cosmetic_variants_keep_processing_species_and_part_distinct(self):
        signature = identities.signature
        self.assertEqual(signature('Côte cheese', 'cheese'), signature('Cote PDO', 'cheese'))
        self.assertEqual(signature('beef rib meat', 'meat'), signature('bovine meat rib', 'meat'))
        self.assertEqual(signature("calf's fillets", 'meat'), signature('filet of calf', 'meat'))
        for left, right in [('active yeast', 'inactive yeast'), ('raw chickpea', 'canned chickpea'),
                            ('canned chickpea', 'dried chickpea'), ('coriander seed', 'coriander leaf'),
                            ('cow milk', 'human milk'), ('pork liver', 'beef liver'),
                            ('head as food', 'head cheese')]:
            with self.subTest(left=left, right=right):
                self.assertNotEqual(signature(left), signature(right))

    def test_negative_scope_precedes_misleading_food_types(self):
        for label, description in [
            ('alpha-amylases', 'enzyme used in digestion'),
            ('used cooking oil', 'discarded vegetable oil'),
            ('breast milk', 'milk produced by humans'),
            ('L’Etivaz', 'hamlet in Switzerland'),
            ('low-fat cheese', 'cheese classification'),
            ('milk or milk based food product', 'food class'),
            ('Class III', 'class for milk used to make hard cheese products in the FMMO'),
            ('cheesemaking', 'economic activity of making cheese'),
            ('Sou', 'ice cream in a square cup manufactured and sold by Lotte'),
            ('antique syrup', 'medicinal syrup used as a laxative'),
            ('head as food', 'edible animal head')]:
            with self.subTest(label=label):
                self.assertIsNotNone(identities.scope_hold(term(label, description)))
        flour = term('self-raising flour', 'flour mixed with chemical leavening')
        self.assertIsNone(identities.scope_hold(flour))
        self.assertTrue(identities.ingredient_evidence(flour, 'grain')[0])

    def test_type_paths_are_bounded_and_recorded_without_overriding_holds(self):
        subject = term('regional ingredient', 'named edible material')
        subject['instance_of'] = ['Q2']
        intermediate = term('regional food', '')
        intermediate['subclass_of'] = ['Q3']
        anchor = term('cheese', '')
        family, path = identities.typed_family(subject, {'Q2': intermediate, 'Q3': anchor})
        self.assertEqual(family, 'cheese')
        self.assertIn('Q1 → Q2 → Q3', path)
        subject['identity_type_evidence'] = path
        self.assertTrue(identities.ingredient_evidence(subject, family)[0])
        subject['description'] = 'chemical compound'
        self.assertIsNotNone(identities.scope_hold(subject))
        intermediate['subclass_of'] = ['Q2']
        self.assertEqual(identities.typed_family(subject, {'Q2': intermediate}), (None, None))

    def test_legacy_duplicate_representative_does_not_mutate_entries(self):
        entries = {
            EXISTING: dict(id=EXISTING, label='Côte cheese', description='cheese', aliases=[],
                           provenance={'origin': 'curated'}),
            RETIRED: dict(id=RETIRED, label='Cote cheese', description='cheese', aliases=[],
                          provenance={'origin': 'imported'})}
        before = copy.deepcopy(entries)
        index = collections.defaultdict(set)
        for eid, entry in entries.items():
            index[('cheese', identities.signature(entry['label'], 'cheese'))].add(eid)
        hits, method = identities.match_existing(term('Cote PDO', 'cheese'), 'cheese', entries, index)
        self.assertEqual(hits, set(entries))
        self.assertEqual(method, 'existing_duplicate_name_family')
        self.assertEqual(identities.representative(hits, entries, 'Cote PDO'), EXISTING)
        self.assertEqual(entries, before)


class IdentityPipelineTest(unittest.TestCase):
    setUp = fixtures.WikidataTest.setUp
    save = fixtures.WikidataTest.save

    def policy(self, decisions):
        self.args.policy = self.root/'policy.json'
        self.args.registry = self.root/'registry.json'
        self.args.registry.write_text(json.dumps({'decisions': []}))
        self.args.policy.write_text(json.dumps(dict(schema_version=1, version='fixture',
            descriptor_checksum=digest(self.args.descriptor), hold_rules=[], decisions=decisions)))

    def test_review_maps_to_new_identity_before_its_qid_is_processed(self):
        self.policy([
            dict(term_id='Q1', action='map', target_term_id='Q2', relationship='related', note='Related source.'),
            dict(term_id='Q2', action='import', relationship='exact', note='Named spice.',
                 canonical=dict(label='new spice', aliases=[], description='Culinary spice.'))])
        prepare(self.args)
        review = load(self.args.output/'review.json')
        self.assertEqual([d['action'] for d in review['decisions']], ['import', 'map'])
        self.assertEqual(review['decisions'][0]['canonical_id'], review['decisions'][1]['canonical_id'])
        self.args.review = self.args.output/'review.json'
        self.args.output = self.root/'built'
        wd.build(self.args)
        result = load(self.args.output/'ingredients.json')
        self.assertEqual(len(result['entries']), 3)
        self.assertEqual(result['entries'][0], self.seed['entries'][0])
        self.assertEqual(result['entries'][1], self.seed['entries'][1])
        self.assertEqual(len(result['entries'][2]['source_mappings']), 2)

    def test_resolved_name_family_replays_with_the_same_uuid(self):
        self.snapshot['entities'][1] = entity('Q2', 'Côte cheese', 102)
        self.snapshot['entities'].append(entity('Q3', 'Cote PDO', 103))
        self.descriptor['revisions']['Q3'] = 103
        for e in self.snapshot['entities'][1:]:
            e['descriptions']['en']['value'] = 'Named cheese from a regional dairy.'
            e['aliases'] = {}
        self.save()
        self.policy([])
        summary = identities.resolve(self.args)
        self.assertEqual(summary['resolutions'], {'import': 1, 'map_new_name_family': 1})
        self.args.policy = self.args.output/'policy.json'
        self.args.output = self.root/'reviewed'
        prepare(self.args)
        first = load(self.args.output/'review.json')
        imports = [d for d in first['decisions'] if d['action'] == 'import']
        self.assertEqual(len(imports), 1)
        mapping = next(d for d in first['decisions'] if d['term_id'] == 'Q3')
        self.assertEqual(mapping['canonical_id'], imports[0]['canonical_id'])
        self.assertEqual(mapping['relationship'], 'related')
        self.args.registry.write_text(json.dumps(first))
        self.args.output = self.root/'resolved-again'
        identities.resolve(self.args)
        self.assertEqual(load(self.args.output/'policy.json'), load(self.args.policy))
        self.args.policy = self.args.output/'policy.json'
        self.args.output = self.root/'reviewed-again'
        prepare(self.args)
        self.assertEqual(load(self.args.output/'review.json'), first)


if __name__ == '__main__':
    unittest.main()
