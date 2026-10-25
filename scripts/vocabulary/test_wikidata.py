"""Offline identity, revision, rank and review-boundary regressions."""
import argparse
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import rename_labels
import wikidata as wd
from foodon import digest, load
from fetch_wikidata import project

EXISTING = '11111111-1111-4111-8111-111111111111'
RETIRED = '22222222-2222-4222-8222-222222222222'
NEW = '33333333-3333-4333-8333-333333333333'


def statement(prop, qid, rank='normal'):
    return dict(rank=rank, mainsnak=dict(property=prop, snaktype='value',
        datavalue=dict(type='wikibase-entityid', value=dict(id=qid))))


def entity(qid, label, rev):
    return dict(id=qid, type='item', lastrevid=rev, labels={'en': {'language': 'en', 'value': label}},
                descriptions={'en': {'language': 'en', 'value': 'A culinary ingredient.'}},
                aliases={'en': [{'language': 'en', 'value': 'unsafe broad synonym'}]},
                claims={'P31': [statement('P31', 'Q25403900')]}, redirect=None)


class WikidataTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.snapshot = dict(schema_version=1, entities=[entity('Q1', 'garlic', 101), entity('Q2', 'new spice', 102)])
        self.seed = dict(schema_version=1, dimension='ingredient', entries=[
            dict(id=EXISTING, label='garlic', aliases=['fresh garlic'], description='Keep authored text.',
                 locale='en', status='active', source_mappings=[], provenance=dict(origin='curated', references=[])),
            dict(id=RETIRED, label='retired ingredient', aliases=[], description='',
                 locale='en', status='retired', source_mappings=[], provenance=dict(origin='curated', references=[]))])
        self.sources = dict(schema_version=1, sources=[])
        self.descriptor = dict(schema_version=1, outside_discovery_entities=[], revisions={'Q1': 101, 'Q2': 102},
            source=dict(id='wikidata-fixture', url='https://www.wikidata.org/wiki/Wikidata:Data_access',
                        version='fixture', checksum='', license='CC0-1.0', attribution='Fixture'), bytes=0)
        self.args = argparse.Namespace(snapshot=self.root/'snapshot.json', descriptor=self.root/'descriptor.json',
            seed=self.root/'seed.json', sources=self.root/'sources.json', review=self.root/'review.json',
            output=self.root/'output')
        self.save()

    def save(self):
        self.args.snapshot.write_text(json.dumps(self.snapshot))
        self.descriptor['bytes'] = self.args.snapshot.stat().st_size
        self.descriptor['source']['checksum'] = digest(self.args.snapshot)
        for key in ['descriptor', 'seed', 'sources']:
            getattr(self.args, key).write_text(json.dumps(getattr(self, key)))

    def decision(self, qid='Q1', eid=EXISTING, **overrides):
        return dict(approved=True, action='map', term_id=qid, revision=self.descriptor['revisions'][qid],
                    canonical_id=eid, relationship='related', note='Reviewed identity/part/form.', **overrides)

    def build(self, decisions):
        review = {**wd.bindings(self.args, self.descriptor), 'decisions': decisions}
        self.args.review.write_text(json.dumps(review))
        return wd.build(self.args)

    def test_extract_accounts_for_every_active_identity_without_approval(self):
        self.seed['entries'][0]['label'] = 'unknown'; self.seed['entries'][0]['aliases'] = []
        self.save()
        with patch('urllib.request.urlopen', side_effect=AssertionError('offline')):
            result = wd.extract(self.args)
        self.assertEqual(result['active_ingredients'], 1)
        rows = load(self.args.output/'seed-name-review.json')
        self.assertEqual(rows[0]['status'], 'no_name_match_in_snapshot')

    def test_lexical_overlap_never_approves_equivalence(self):
        wd.extract(self.args)
        review = load(self.args.output/'review.json')
        self.assertEqual(len(review['decisions']), 1)
        self.assertFalse(review['decisions'][0]['approved'])
        self.assertIsNone(review['decisions'][0]['relationship'])

    def test_build_preserves_original_wording_and_retirement(self):
        self.build([self.decision()])
        result = load(self.args.output/'ingredients.json')
        for before, after in zip(self.seed['entries'], result['entries']):
            for key in ['id', 'label', 'aliases', 'description', 'status', 'locale']:
                self.assertEqual(before[key], after[key])
        self.assertEqual(result['entries'][0]['aliases'], ['fresh garlic'])
        self.assertEqual(result['entries'][1], self.seed['entries'][1])

    def test_import_requires_explicit_wording_and_retains_uuid(self):
        d = self.decision('Q2', NEW);d.update(action='import', relationship='exact',
            canonical=dict(label='new spice', aliases=[], description='Reviewed culinary wording.'))
        self.build([d]); first = (self.args.output/'ingredients.json').read_bytes()
        self.args.output = self.root/'second';self.build([d])
        self.assertEqual(first, (self.args.output/'ingredients.json').read_bytes())
        self.assertEqual(load(self.args.output/'ingredients.json')['entries'][-1]['id'], NEW)

    def test_import_name_cannot_duplicate_an_existing_alias(self):
        d=self.decision('Q2',NEW);d.update(action='import', relationship='exact',
            canonical=dict(label='fresh garlic', aliases=[], description=''))
        with self.assertRaisesRegex(ValueError, 'collides'):self.build([d])

    def test_snapshot_checksum_and_revision_are_independently_verified(self):
        self.args.snapshot.write_text('{}')
        with self.assertRaisesRegex(ValueError, 'checksum'):wd.context(self.args)
        self.snapshot['entities'][0]['lastrevid'] = 999;self.save()
        with self.assertRaisesRegex(ValueError, 'revision'):wd.context(self.args)

    def test_review_is_bound_to_base_and_descriptor(self):
        review={**wd.bindings(self.args,self.descriptor),'decisions':[self.decision()]}
        self.args.review.write_text(json.dumps(review));self.seed['entries'][0]['description']='Changed';self.save()
        with self.assertRaisesRegex(ValueError,'Stale'):wd.build(self.args)

    def test_decision_cannot_silently_follow_a_new_entity_revision(self):
        d=self.decision();d['revision']=999
        with self.assertRaisesRegex(ValueError,'revision'):self.build([d])

    def test_unresolved_redirect_taxon_and_disambiguation_are_blocked(self):
        for flag in ['redirect','taxon','disambiguation']:
            with self.subTest(flag=flag):
                self.snapshot['entities'][0]=entity('Q1','garlic',101)
                e=self.snapshot['entities'][0]
                if flag=='redirect':e['redirect']='Q2'
                else:e['claims']['P31']=[statement('P31','Q16521' if flag=='taxon' else 'Q4167410')]
                self.save()
                with self.assertRaises(ValueError):self.build([self.decision()])

    def test_deprecated_and_non_value_statements_do_not_define_scope(self):
        e=entity('Q1','garlic',101)
        e['claims']['P31']=[statement('P31','Q16521','deprecated'),statement('P31','Q25403900')]
        e['claims']['P31'].append(dict(rank='normal',mainsnak=dict(property='P31',snaktype='novalue')))
        self.assertEqual(wd.item_values(e,'P31'), ['Q25403900'])

    def test_preferred_statements_supersede_normal_statements(self):
        e=entity('Q1','garlic',101)
        e['claims']['P31'] += [statement('P31','Q2','preferred')]
        self.assertEqual(wd.item_values(e,'P31'), ['Q2'])

    def test_approval_boolean_rationale_and_relationship_are_required(self):
        for key,value in [('approved','true'),('note',''),('relationship',None)]:
            d=self.decision();d[key]=value
            with self.subTest(key=key),self.assertRaises(ValueError):self.build([d])

    def test_exact_source_identity_cannot_be_assigned_to_two_canonicals(self):
        d=self.decision();d['relationship']='exact'
        other=dict(d,canonical_id=RETIRED)
        with self.assertRaisesRegex(ValueError,'distinct canonical'):self.build([d,other])

    def test_outside_scope_entity_cannot_be_imported(self):
        self.descriptor['outside_discovery_entities']=['Q2'];self.save()
        d=self.decision('Q2',NEW);d.update(action='import',relationship='exact',canonical=dict(label='new spice',aliases=[],description=''))
        with self.assertRaisesRegex(ValueError,'eligible'):self.build([d])

    def test_map_cannot_replace_authored_fields_and_output_cannot_overwrite(self):
        d=self.decision();d['canonical']={'label':'replacement'}
        with self.assertRaisesRegex(ValueError,'preserve'):self.build([d])
        self.build([self.decision()])
        with self.assertRaisesRegex(ValueError,'exists'):self.build([self.decision()])

    def test_collection_handles_wikibase_empty_maps_and_preserves_evidence(self):
        raw=entity('Q1','garlic',101);raw['aliases']=[];raw['descriptions']=[];raw['labels']=[]
        raw['claims']['P31'][0]['references']=[{'hash':'evidence'}]
        result=project(raw,101)
        self.assertEqual(result['aliases'],{})
        self.assertEqual(result['descriptions'],{})
        self.assertEqual(result['labels'],{})
        self.assertEqual(result['claims']['P31'][0]['references'],[{'hash':'evidence'}])

class NameReviewTest(unittest.TestCase):
    setUp = WikidataTest.setUp
    save = WikidataTest.save

    def prepare(self, overrides=None):
        from review_wikidata import prepare
        self.args.policy=self.root/'policy.json'
        self.args.registry=self.root/'registry.json'
        self.args.policy.write_text(json.dumps(dict(schema_version=1,version='fixture',
            descriptor_checksum=digest(self.args.descriptor),
            hold_rules=[dict(field='combined',status='hold_artifact',pattern=r'\bmuseum\b')],
            decisions=overrides or [])))
        return prepare(self.args)

    def test_name_policy_uses_related_only_and_holds_new_identities(self):
        self.prepare()
        review=load(self.args.output/'review.json')
        self.assertEqual(len(review['decisions']),1)
        self.assertEqual(review['decisions'][0]['relationship'],'related')
        rows=load(self.args.output/'source-name-review.json')
        self.assertEqual(rows[1]['status'],'hold_new_identity_review')
        self.assertEqual(len(load(self.args.output/'seed-name-review.json')),1)

    def test_food_hierarchy_membership_does_not_admit_museum_artifacts(self):
        self.snapshot['entities'][0]['descriptions']['en']['value']='egg at a museum'
        self.save(); self.prepare()
        self.assertEqual(load(self.args.output/'review.json')['decisions'],[])
        self.assertEqual(load(self.args.output/'source-name-review.json')[0]['status'],'hold_artifact')

    def test_ambiguous_alias_overlap_cannot_become_an_automatic_mapping(self):
        self.seed['entries'][1]['status']='active'
        self.seed['entries'][1]['aliases']=['unsafe broad synonym']
        self.save();self.prepare()
        rows=load(self.args.output/'source-name-review.json')
        self.assertTrue(all(r['status']=='hold_alias_or_identity_ambiguity' for r in rows))

    def test_explicit_import_reuses_registry_identity(self):
        override=dict(term_id='Q2',action='import',relationship='exact',note='Specific spice.',
                      canonical=dict(label='new spice',aliases=[],description='Culinary spice.'))
        self.prepare([override])
        first=load(self.args.output/'review.json')
        self.args.registry.write_text(json.dumps(first));self.args.output=self.root/'second'
        self.prepare([override])
        self.assertEqual(first,load(self.args.output/'review.json'))

    def test_policy_cannot_silently_drop_persisted_imports(self):
        self.args.registry=self.root/'registry.json'
        self.args.registry.write_text(json.dumps({'decisions':[dict(action='import',term_id='Q2',canonical_id=NEW)]}))
        with self.assertRaisesRegex(ValueError,'drops a persisted import'):self.prepare()


class MaintainedWikidataTest(unittest.TestCase):
    def test_approved_ledger_matches_canonical_seed_without_upstream_download(self):
        descriptor=load(wd.ROOT/'data/vocabulary/adapters/wikidata.json')
        review=load(wd.ROOT/'data/vocabulary/adapters/wikidata-cooking-review.json')
        seed=rename_labels.restore_baseline(load(wd.ROOT/'data/vocabulary/ingredients.json'))
        manifest=load(wd.ROOT/'data/vocabulary/sources.json')
        self.assertEqual(wd.validate(seed,manifest),[])
        self.assertEqual(review['descriptor_checksum'],digest(wd.ROOT/'data/vocabulary/adapters/wikidata.json'))
        entries={e['id']:e for e in seed['entries']}
        for d in review['decisions']:
            self.assertEqual(d['revision'],descriptor['revisions'][d['term_id']])
            self.assertIn(dict(source=descriptor['source']['id'],external_id=d['term_id'],relationship=d['relationship']),
                          entries[d['canonical_id']]['source_mappings'])
            if d['action']=='import':
                for key,value in d['canonical'].items():
                    current=entries[d['canonical_id']][key]
                    # Later alias-only review appends to the exact source stage.
                    if key=='aliases' and entries[d['canonical_id']]['status']=='active':current=current[:len(value)]
                    self.assertEqual(current,value)


if __name__ == '__main__':
    unittest.main()
