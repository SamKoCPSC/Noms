"""Offline bulk-archive, approval, culinary-scope and UUID regressions."""
import argparse
import copy
import csv
import io
import json
from pathlib import Path
import tempfile
import unittest
import zipfile

from foodon import digest, load
import review_usda
import usda

EXISTING = '11111111-1111-4111-8111-111111111111'
RETIRED = '22222222-2222-4222-8222-222222222222'
NEW = '33333333-3333-4333-8333-333333333333'


class UsdaTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.args = argparse.Namespace(archives=self.root, descriptor=self.root/'descriptor.json',
            seed=self.root/'seed.json', sources=self.root/'sources.json', review=self.root/'review.json',
            policy=self.root/'policy.json', registry=self.root/'registry.json', output=self.root/'output')
        self.foods = [dict(fdc_id='1', data_type='foundation_food', description='Garlic, raw',
                          food_category_id='2', publication_date='2026-04-30'),
                      dict(fdc_id='2', data_type='foundation_food', description='Spices, new spice',
                          food_category_id='2', publication_date='2026-04-30'),
                      dict(fdc_id='3', data_type='foundation_food', description='Spices, new spice',
                          food_category_id='2', publication_date='2026-04-30'),
                      dict(fdc_id='4', data_type='sample_food', description='Sample garlic',
                          food_category_id='2', publication_date='2026-04-30')]
        self.seed = dict(schema_version=1, dimension='ingredient', entries=[
            dict(id=EXISTING, label='garlic', aliases=['fresh garlic'], description='Authored definition.',
                 locale='en', status='active', source_mappings=[], provenance=dict(origin='curated', references=[])),
            dict(id=RETIRED, label='old ingredient', aliases=[], description='', locale='en', status='retired',
                 source_mappings=[], provenance=dict(origin='curated', references=[]))])
        self.sources = dict(schema_version=1, sources=[])
        self.save()

    def save(self):
        archive = self.root/'fixture.zip'
        with zipfile.ZipFile(archive, 'w') as z:
            for filename, rows, fields in [
                ('food.csv', self.foods, list(self.foods[0])),
                ('food_category.csv', [dict(id='2', code='0200', description='Spices and Herbs')],
                 ['id','code','description']),
                # Missing Foundation metadata is valid in the published archive.
                ('foundation_food.csv', [dict(fdc_id='1', NDB_number='1001')], ['fdc_id','NDB_number'])]:
                out = io.StringIO(newline=''); writer = csv.DictWriter(out, fieldnames=fields)
                writer.writeheader(); writer.writerows(rows); z.writestr('fixture/'+filename, out.getvalue())
        with zipfile.ZipFile(archive) as z:
            members = [dict(name=i.filename, bytes=i.file_size, crc32=f'{i.CRC:08x}') for i in z.infolist()]
        self.descriptor = dict(schema_version=1, archives=[dict(key='foundation', local_filename='fixture.zip',
            bytes=archive.stat().st_size, data_type='foundation_food', eligible_rows=3, members=members,
            source=dict(id='usda-fixture', url='https://fdc.nal.usda.gov/fixture.zip', version='fixture',
                        checksum=digest(archive), license='CC0-1.0', attribution='USDA fixture'))])
        for name in ['descriptor','seed','sources']:
            getattr(self.args, name).write_text(json.dumps(getattr(self, name)))
        self.policy = dict(schema_version=1, version='fixture', descriptor_checksum=digest(self.args.descriptor),
            held_categories={}, hold_rules=[], association_only_drop_parts=['raw'], association_names={},
            no_import_patterns=[], case_sensitive_no_import_patterns=[], max_unmatched_parts=2,
            allow_import_candidates=False, overrides={})
        self.save_policy()

    def save_policy(self):
        self.args.policy.write_text(json.dumps(self.policy))

    def decision(self, **values):
        terms, _ = usda.read_archives(self.root, self.descriptor)
        return dict(approved=True, action='map', record_id='foundation:1',
            record_checksum=terms['foundation:1']['record_checksum'], canonical_id=EXISTING,
            relationship='related', note='Reviewed source association.', **values)

    def build(self, decisions):
        self.args.review.write_text(json.dumps(dict(usda.bindings(self.args), decisions=decisions)))
        return usda.build(self.args)

    def test_only_food_rows_are_extracted_and_optional_ndb_is_not_identity(self):
        usda.extract(self.args)
        rows = load(self.args.output/'candidates.json')['terms']
        self.assertEqual([r['fdc_id'] for r in rows], ['1','2','3'])
        self.assertEqual(rows[1]['ndb_number'], '')
        self.assertEqual(rows[1]['data_type'], 'foundation_food')
        self.assertEqual(rows[1]['source'], 'usda-fixture')
        self.assertEqual(load(self.args.output/'review.json')['decisions'], [])

    def test_archive_and_member_pins_are_verified(self):
        pin = self.descriptor['archives'][0]
        pin['members'][0]['crc32'] = '00000000'
        with self.assertRaisesRegex(ValueError, 'member identity'):
            usda.read_archives(self.root, self.descriptor)
        pin['source']['checksum'] = '0'*64
        with self.assertRaisesRegex(ValueError, 'checksum'):
            usda.read_archives(self.root, self.descriptor)

    def test_duplicate_fdc_ids_fail_instead_of_silently_overwriting(self):
        self.foods[2]['fdc_id'] = '2'; self.save()
        with self.assertRaisesRegex(ValueError, 'repeated FDC'):
            usda.read_archives(self.root, self.descriptor)

    def test_wrong_data_type_or_eligible_count_fails(self):
        self.descriptor['archives'][0]['data_type'] = 'branded_food'
        with self.assertRaisesRegex(ValueError, 'archive pin'):
            usda.read_archives(self.root, self.descriptor)
        self.descriptor['archives'][0]['data_type'] = 'foundation_food'
        self.descriptor['archives'][0]['eligible_rows'] = 4
        with self.assertRaisesRegex(ValueError, 'record count'):
            usda.read_archives(self.root, self.descriptor)

    def test_map_preserves_wording_ids_and_retirements(self):
        self.build([self.decision()])
        after = load(self.args.output/'ingredients.json')['entries']
        for original, result in zip(self.seed['entries'], after):
            for key in ['id','label','aliases','description','locale','status']:
                self.assertEqual(original[key], result[key])
        self.assertEqual(after[1], self.seed['entries'][1])
        self.assertEqual(after[0]['source_mappings'][0]['external_id'], '1')

    def test_stale_base_and_changed_row_evidence_are_independent_gates(self):
        d = self.decision(); d['record_checksum'] = '0'*64
        with self.assertRaisesRegex(ValueError, 'row evidence'): self.build([d])
        self.args.review.write_text(json.dumps(dict(usda.bindings(self.args), decisions=[self.decision()])))
        self.seed['entries'][0]['description'] = 'Changed'; self.args.seed.write_text(json.dumps(self.seed))
        with self.assertRaisesRegex(ValueError, 'Stale'): usda.build(self.args)

    def test_approval_unknown_fields_and_retired_target_are_rejected(self):
        for key, value in [('approved','true'), ('canonical',{'label':'overwrite'}),
                           ('canonical_id',RETIRED), ('unexpected','field')]:
            d = self.decision(); d[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError): self.build([d])

    def test_source_id_cannot_silently_change_metadata(self):
        source = copy.deepcopy(self.descriptor['archives'][0]['source']); source['version'] = 'changed'
        self.sources['sources'] = [source]; self.args.sources.write_text(json.dumps(self.sources))
        with self.assertRaisesRegex(ValueError, 'different metadata'): self.build([self.decision()])

    def test_import_requires_saved_uuid_wording_and_no_alias_collision(self):
        d = self.decision(); d.update(action='import', canonical_id=NEW, relationship='exact',
            canonical=dict(label='new spice', aliases=['fresh garlic'], description='Culinary spice.'))
        terms, _ = usda.read_archives(self.root, self.descriptor)
        d.update(record_id='foundation:2', record_checksum=terms['foundation:2']['record_checksum'])
        with self.assertRaisesRegex(ValueError, 'collides'): self.build([d])
        d['canonical']['aliases'] = []; self.build([d])
        first = (self.args.output/'ingredients.json').read_bytes()
        self.args.output = self.root/'second'; self.build([d])
        self.assertEqual(first, (self.args.output/'ingredients.json').read_bytes())

    def test_repeated_records_share_one_reviewed_uuid_and_rerun_is_stable(self):
        self.policy['overrides']['foundation:2'] = dict(action='import', relationship='exact',
            canonical=dict(label='new spice', aliases=[], description='Culinary spice.'), note='Reviewed spice.')
        self.save_policy(); review_usda.prepare(self.args)
        review = load(self.args.output/'review.json')
        imports = [d for d in review['decisions'] if d['action']=='import']
        self.assertEqual(len(imports), 1)
        third = next(d for d in review['decisions'] if d['record_id']=='foundation:3')
        self.assertEqual(third['canonical_id'], imports[0]['canonical_id'])
        self.assertEqual(third['relationship'], 'related')
        self.assertEqual(review['decisions'][0]['action'], 'import')
        self.args.registry.write_text(json.dumps(review)); self.args.output = self.root/'again'
        review_usda.prepare(self.args)
        self.assertEqual(load(self.args.output/'review.json'), review)
        self.policy['overrides'] = {}; self.save_policy(); self.args.output = self.root/'dropped'
        with self.assertRaisesRegex(ValueError, 'drops a persisted'): review_usda.prepare(self.args)

    def test_unknown_names_are_held_without_explicit_import_approval(self):
        summary = review_usda.prepare(self.args)
        self.assertEqual(summary['imports'], 0)
        review = load(self.args.output/'review.json')
        self.assertEqual(review['decisions'][0]['relationship'], 'related')
        self.assertEqual(len(review['decisions']), 1)

    def test_morphology_preserves_food_parts_and_forms(self):
        signature = review_usda.signature
        self.assertEqual(signature('Nuts, brazilnuts'), signature('brazil nut nuts'))
        self.assertEqual(signature('Peas, dried'), signature('dry pea'))
        for first, second in [('coriander leaf','coriander seed'), ('canned apple','dried apple'),
                              ('raw beef','cooked beef'), ('salted butter','unsalted butter'),
                              ('pork liver','beef liver'), ('hulled hemp seed','hemp seed with hull')]:
            self.assertNotEqual(signature(first), signature(second))

    def test_maintained_policy_holds_complete_meals_products_and_analytical_names(self):
        policy = load(usda.ROOT/'data/vocabulary/adapters/usda-cooking-policy.json')
        for label, category in [('Soup, mushroom with beef stock, canned', '6'),
                                ('Pillsbury, cinnamon rolls', '18'),
                                ('Soy protein concentrate, produced by acid wash', '16'),
                                ('Milk shakes, thick chocolate', '1'),
                                ('Turkey and gravy, frozen', '5')]:
            with self.subTest(label=label):
                self.assertIsNotNone(review_usda.scope_hold(dict(label=label, category_id=category), policy))
        self.assertIsNone(review_usda.scope_hold(
            dict(label='Soup, chicken broth, ready-to-serve', category_id='6'), policy))
        self.assertFalse(policy['allow_import_candidates'])


if __name__ == '__main__':
    unittest.main()
